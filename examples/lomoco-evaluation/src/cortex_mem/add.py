import json
import os
import subprocess
import time
# Removed ThreadPoolExecutor import - using sequential processing instead
from pathlib import Path

from dotenv import load_dotenv
from tqdm import tqdm

from .config_utils import check_openai_config, get_config_value, validate_config

load_dotenv()


class CortexMemAdd:
    def __init__(self, data_path=None, batch_size=2, config_path=None):
        self.batch_size = batch_size
        self.data_path = data_path
        self.data = None
        self.config_path = config_path or self._find_config_file()

        # Validate config file
        if not validate_config(self.config_path):
            raise ValueError(f"Invalid config file: {self.config_path}")

        # Check OpenAI configuration
        if not check_openai_config(self.config_path):
            raise ValueError(
                f"OpenAI configuration not properly set in {self.config_path}"
            )

        if data_path:
            self.load_data()

    def _find_config_file(self):
        """Find config.toml file in standard locations"""
        # Check current directory
        if os.path.exists("config.toml"):
            return "config.toml"

        # Check parent directories
        current_dir = Path.cwd()
        for parent in current_dir.parents:
            config_file = parent / "config.toml"
            if config_file.exists():
                return str(config_file)

        # Check examples directory
        examples_config = (
            Path(__file__).parent.parent.parent.parent / "examples" / "config.toml"
        )
        if examples_config.exists():
            return str(examples_config)

        # Check project root
        project_root = Path(__file__).parent.parent.parent.parent.parent
        config_file = project_root / "config.toml"
        if config_file.exists():
            return str(config_file)

        raise FileNotFoundError("Could not find config.toml file")

    def load_data(self):
        with open(self.data_path, "r") as f:
            self.data = json.load(f)
        return self.data

    def _run_cortex_mem_cli(self, args):
        """Run cortex-mem-cli command"""
        # First, ensure the project is built
        build_cmd = ["cargo", "build", "-p", "cortex-mem-cli"]
        subprocess.run(build_cmd, capture_output=True, text=True)

        # Use absolute path for config file to avoid path resolution issues
        config_path = os.path.abspath(self.config_path)

        # Run the CLI with absolute config file path
        cmd = ["cargo", "run", "-p", "cortex-mem-cli", "--quiet", "--"]
        cmd.extend(["--config", config_path])
        cmd.extend(args)

        try:
            # Use project root as working directory (examples/lomoco-evaluation -> cortex-mem)
            project_root = Path(__file__).parent.parent.parent.parent
            
            # Use UTF-8 encoding to avoid GBK codec errors on Windows
            result = subprocess.run(
                cmd, 
                capture_output=True, 
                text=True, 
                encoding='utf-8',
                cwd=str(project_root)
            )

            if result.returncode != 0:
                print(f"CLI command failed: {result.stderr}")

            return result.returncode == 0, result.stdout, result.stderr
        except Exception as e:
            print(f"Error running CLI: {e}")
            return False, "", str(e)

    def add_memory(
        self, user_id, content, memory_type="conversational"
    ):
        """Add a memory using cortex-mem-cli"""
        args = [
            "add",
            "--content",
            content,
            "--user-id",
            user_id,
            "--memory-type",
            memory_type,
        ]

        success, stdout, stderr = self._run_cortex_mem_cli(args)

        if not success:
            print(f"Failed to add memory for user {user_id}: {stderr}")

        return success

    def add_memories_for_speaker(self, speaker, messages, timestamp, desc):
        """Add memories for a speaker"""
        for i in tqdm(range(0, len(messages), self.batch_size), desc=desc):
            batch_messages = messages[i : i + self.batch_size]

            # Combine batch messages into single content
            content = "\n".join([msg.get("content", "") for msg in batch_messages])

            # Add timestamp as metadata
            metadata = f"Timestamp: {timestamp}"
            content_with_metadata = f"{metadata}\n{content}"

            self.add_memory(
                speaker,
                content_with_metadata,
                memory_type="conversational",
            )

    def process_conversation(self, item, idx):
        """Process a single conversation"""
        conversation = item["conversation"]
        speaker_a = conversation["speaker_a"]
        speaker_b = conversation["speaker_b"]

        speaker_a_user_id = f"{speaker_a}_{idx}"
        speaker_b_user_id = f"{speaker_b}_{idx}"

        # Note: Cortex Mem doesn't have a delete_all function in CLI
        # We'll rely on unique user IDs for each conversation

        for key in conversation.keys():
            if key in ["speaker_a", "speaker_b"] or "date" in key or "timestamp" in key:
                continue

            date_time_key = key + "_date_time"
            timestamp = conversation[date_time_key]
            chats = conversation[key]

            messages = []
            messages_reverse = []
            for chat in chats:
                if chat["speaker"] == speaker_a:
                    messages.append(
                        {"role": "user", "content": f"{speaker_a}: {chat['text']}"}
                    )
                    messages_reverse.append(
                        {"role": "assistant", "content": f"{speaker_a}: {chat['text']}"}
                    )
                elif chat["speaker"] == speaker_b:
                    messages.append(
                        {"role": "assistant", "content": f"{speaker_b}: {chat['text']}"}
                    )
                    messages_reverse.append(
                        {"role": "user", "content": f"{speaker_b}: {chat['text']}"}
                    )
                else:
                    raise ValueError(f"Unknown speaker: {chat['speaker']}")

            # Add memories for both speakers
            self.add_memories_for_speaker(
                speaker_a_user_id,
                messages,
                timestamp,
                f"Adding Memories for {speaker_a}",
            )
            self.add_memories_for_speaker(
                speaker_b_user_id,
                messages_reverse,
                timestamp,
                f"Adding Memories for {speaker_b}",
            )

        print(f"Messages added successfully for conversation {idx}")

    def process_all_conversations(self, max_workers=5):
        """Process all conversations"""
        if not self.data:
            raise ValueError(
                "No data loaded. Please set data_path and call load_data() first."
            )

        # Process conversations sequentially to avoid concurrency issues
        for idx, item in enumerate(self.data):
            self.process_conversation(item, idx)
