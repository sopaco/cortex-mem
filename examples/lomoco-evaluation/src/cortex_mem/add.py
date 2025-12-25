import json
import os
import subprocess
import time
import logging
from pathlib import Path
from typing import Tuple

from tqdm import tqdm

from .config_utils import check_openai_config, get_config_value, validate_config

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


class CortexMemAdd:
    def __init__(self, data_path=None, batch_size=2, config_path=None):
        self.batch_size = batch_size
        self.data_path = data_path
        self.data = None
        self.config_path = config_path or self._find_config_file()

        # Track statistics
        self.stats = {
            "total_conversations": 0,
            "successful_conversations": 0,
            "failed_conversations": 0,
            "total_memories": 0,
            "successful_memories": 0,
            "failed_memories": 0
        }

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
        if not self.data_path:
            raise ValueError("data_path not set")
        with open(self.data_path, "r") as f:
            self.data = json.load(f)
        return self.data

    def _run_cortex_mem_cli(self, args, max_retries=3):
        """Run cortex-mem-cli command with retry logic"""
        for attempt in range(max_retries):
            try:
                # First, ensure the project is built (only on first attempt)
                if attempt == 0:
                    build_cmd = ["cargo", "build", "-p", "cortex-mem-cli", "--release"]
                    result = subprocess.run(build_cmd, capture_output=True, text=True, timeout=300)
                    if result.returncode != 0:
                        logger.warning(f"Build warning: {result.stderr}")

                # Use absolute path for config file to avoid path resolution issues
                config_path = os.path.abspath(self.config_path)

                # Run the CLI with absolute config file path
                cmd = ["cargo", "run", "-p", "cortex-mem-cli", "--quiet", "--"]
                cmd.extend(["--config", config_path])
                cmd.extend(args)

                # Use project root as working directory (examples/lomoco-evaluation -> cortex-mem)
                project_root = Path(__file__).parent.parent.parent.parent
                
                # Use UTF-8 encoding to avoid GBK codec errors on Windows
                result = subprocess.run(
                    cmd, 
                    capture_output=True, 
                    text=True, 
                    encoding='utf-8',
                    timeout=60,
                    cwd=str(project_root)
                )

                if result.returncode != 0:
                    if attempt < max_retries - 1:
                        logger.warning(f"CLI command failed (attempt {attempt+1}/{max_retries}): {result.stderr}")
                        time.sleep(2 ** attempt)  # Exponential backoff
                        continue
                    else:
                        logger.error(f"CLI command failed after {max_retries} attempts: {result.stderr}")

                return result.returncode == 0, result.stdout, result.stderr
            except subprocess.TimeoutExpired:
                logger.warning(f"CLI command timed out (attempt {attempt+1}/{max_retries})")
                if attempt < max_retries - 1:
                    time.sleep(2 ** attempt)
                    continue
                return False, "", "Command timed out"
            except Exception as e:
                logger.error(f"Error running CLI (attempt {attempt+1}/{max_retries}): {e}")
                if attempt < max_retries - 1:
                    time.sleep(2 ** attempt)
                    continue
                return False, "", str(e)

        return False, "", "Max retries exceeded"

    def add_memory(
        self, user_id, content, memory_type="conversational"
    ):
        """Add a memory using cortex-mem-cli with error tracking"""
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
            logger.error(f"Failed to add memory for user {user_id}: {stderr}")
        else:
            logger.debug(f"Successfully added memory for user {user_id}")

        return success

    def add_memories_for_speaker(self, speaker, messages, timestamp, desc):
        """Add memories for a speaker with error tracking"""
        total_batches = (len(messages) + self.batch_size - 1) // self.batch_size
        failed_batches = 0
        
        for i in tqdm(range(0, len(messages), self.batch_size), desc=desc):
            batch_messages = messages[i : i + self.batch_size]

            # Combine batch messages into single content
            content = "\n".join([msg.get("content", "") for msg in batch_messages])

            # Add timestamp as metadata
            metadata = f"Timestamp: {timestamp}"
            content_with_metadata = f"{metadata}\n{content}"

            # Add memory with error tracking
            success = self.add_memory(
                speaker,
                content_with_metadata,
                memory_type="conversational",
            )

            if success:
                self.stats["successful_memories"] += 1
            else:
                self.stats["failed_memories"] += 1
                failed_batches += 1

            self.stats["total_memories"] += 1
            
            # Small delay between batches to avoid rate limiting
            time.sleep(0.3)
        
        if failed_batches > 0:
            logger.warning(f"{failed_batches}/{total_batches} batches failed for {speaker}")

    def process_conversation(self, item, idx):
        """Process a single conversation with error handling"""
        try:
            conversation = item.get("conversation", {})
            speaker_a = conversation.get("speaker_a", "SpeakerA")
            speaker_b = conversation.get("speaker_b", "SpeakerB")

            speaker_a_user_id = f"{speaker_a}_{idx}"
            speaker_b_user_id = f"{speaker_b}_{idx}"

            # Note: Cortex Mem doesn't have a delete_all function in CLI
            # We'll rely on unique user IDs for each conversation

            for key in conversation.keys():
                if key in ["speaker_a", "speaker_b"] or "date" in key or "timestamp" in key:
                    continue

                date_time_key = key + "_date_time"
                timestamp = conversation.get(date_time_key, "2024-01-01 00:00:00")
                chats = conversation[key]

                messages = []
                messages_reverse = []
                for chat in chats:
                    speaker = chat.get("speaker", "")
                    text = chat.get("text", "")
                    
                    if speaker == speaker_a:
                        messages.append(
                            {"role": "user", "content": f"{speaker_a}: {text}"}
                        )
                        messages_reverse.append(
                            {"role": "assistant", "content": f"{speaker_a}: {text}"}
                        )
                    elif speaker == speaker_b:
                        messages.append(
                            {"role": "assistant", "content": f"{speaker_b}: {text}"}
                        )
                        messages_reverse.append(
                            {"role": "user", "content": f"{speaker_b}: {text}"}
                        )
                    else:
                        logger.warning(f"Unknown speaker: {speaker}")

                # Add memories for both speakers
                self.add_memories_for_speaker(
                    speaker_a_user_id,
                    messages,
                    timestamp,
                    f"Adding Memories for {speaker_a}",
                )
                
                time.sleep(0.3)  # Small delay between speakers
                
                self.add_memories_for_speaker(
                    speaker_b_user_id,
                    messages_reverse,
                    timestamp,
                    f"Adding Memories for {speaker_b}",
                )

            self.stats["successful_conversations"] += 1
            logger.info(f"✅ Successfully processed conversation {idx}")

        except Exception as e:
            self.stats["failed_conversations"] += 1
            logger.error(f"❌ Failed to process conversation {idx}: {e}")
            # Continue processing other conversations

        self.stats["total_conversations"] += 1

    def process_all_conversations(self, max_workers=1):
        """Process all conversations sequentially for stability"""
        if not self.data:
            raise ValueError(
                "No data loaded. Please set data_path and call load_data() first."
            )

        logger.info(f"Starting to process {len(self.data)} conversations...")
        
        # Process conversations sequentially for stability
        for idx, item in enumerate(self.data):
            self.process_conversation(item, idx)
            
            # Small delay between conversations to avoid overwhelming the system
            time.sleep(0.5)
        
        # Print summary
        self.print_summary()

    def print_summary(self):
        """Print processing summary"""
        print("\n" + "=" * 60)
        print("📊 PROCESSING SUMMARY")
        print("=" * 60)
        print(f"Total Conversations:      {self.stats['total_conversations']}")
        print(f"Successful:               {self.stats['successful_conversations']}")
        print(f"Failed:                   {self.stats['failed_conversations']}")
        if self.stats['total_conversations'] > 0:
            print(f"Success Rate:             {self.stats['successful_conversations']/self.stats['total_conversations']*100:.1f}%")
        print(f"\nTotal Memories:           {self.stats['total_memories']}")
        print(f"Successful:               {self.stats['successful_memories']}")
        print(f"Failed:                   {self.stats['failed_memories']}")
        if self.stats['total_memories'] > 0:
            print(f"Success Rate:             {self.stats['successful_memories']/self.stats['total_memories']*100:.1f}%")
        print("=" * 60 + "\n")
