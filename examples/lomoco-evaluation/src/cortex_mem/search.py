import json
import os
import subprocess
import time
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from dotenv import load_dotenv
from jinja2 import Template
from openai import OpenAI
from prompts import ANSWER_PROMPT
from tqdm import tqdm

from .config_utils import (
    validate_config,
    check_openai_config,
    get_config_value
)

load_dotenv()


class CortexMemSearch:
    def __init__(self, output_path="results.json", top_k=10, config_path=None):
        self.top_k = top_k
        self.openai_client = OpenAI()
        self.results = defaultdict(list)
        self.output_path = output_path
        self.config_path = config_path or self._find_config_file()
        self.ANSWER_PROMPT = ANSWER_PROMPT
        
        # Validate config file
        if not validate_config(self.config_path):
            raise ValueError(f"Invalid config file: {self.config_path}")
        
        # Check OpenAI configuration
        if not check_openai_config(self.config_path):
            raise ValueError(f"OpenAI configuration not properly set in {self.config_path}")

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
        examples_config = Path(__file__).parent.parent.parent.parent / "examples" / "config.toml"
        if examples_config.exists():
            return str(examples_config)
        
        # Check project root
        project_root = Path(__file__).parent.parent.parent.parent.parent
        config_file = project_root / "config.toml"
        if config_file.exists():
            return str(config_file)
        
        raise FileNotFoundError("Could not find config.toml file")

    def _run_cortex_mem_cli(self, args):
        """Run cortex-mem-cli command"""
        # First, ensure the project is built
        build_cmd = ["cargo", "build", "--bin", "cortex-mem-cli"]
        subprocess.run(build_cmd, capture_output=True, text=True)
        
        # Run the CLI with original config file
        cmd = ["cargo", "run", "--bin", "cortex-mem-cli", "--quiet", "--"]
        cmd.extend(["--config", self.config_path])
        cmd.extend(args)
        
        try:
            # Use project root as working directory
            project_root = Path(__file__).parent.parent.parent.parent.parent
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                cwd=str(project_root)
            )
            
            if result.returncode != 0:
                print(f"CLI command failed: {result.stderr}")
            
            return result.returncode == 0, result.stdout, result.stderr
        except Exception as e:
            print(f"Error running CLI: {e}")
            return False, "", str(e)

    def search_memory(self, user_id, query, max_retries=3, retry_delay=1):
        """Search for memories using cortex-mem-cli"""
        start_time = time.time()
        retries = 0
        
        while retries < max_retries:
            try:
                # Build search command
                args = [
                    "search",
                    "--query", query,
                    "--user-id", user_id,
                    "--limit", str(self.top_k)
                ]
                
                success, stdout, stderr = self._run_cortex_mem_cli(args)
                
                if not success:
                    raise RuntimeError(f"Search failed: {stderr}")
                
                # Parse the output (assuming JSON output from CLI)
                # This is a simplified parser - adjust based on actual CLI output format
                memories = []
                if stdout.strip():
                    try:
                        # Try to parse as JSON
                        result_data = json.loads(stdout)
                        if isinstance(result_data, list):
                            for item in result_data:
                                memory = {
                                    "memory": item.get("content", ""),
                                    "timestamp": item.get("created_at", ""),
                                    "score": item.get("score", 0.0)
                                }
                                memories.append(memory)
                    except json.JSONDecodeError:
                        # If not JSON, parse line by line
                        lines = stdout.strip().split('\n')
                        for line in lines:
                            if line.strip():
                                memory = {
                                    "memory": line.strip(),
                                    "timestamp": "",
                                    "score": 0.0
                                }
                                memories.append(memory)
                
                end_time = time.time()
                return memories, None, end_time - start_time
                
            except Exception as e:
                print(f"Search error: {e}, retrying...")
                retries += 1
                if retries >= max_retries:
                    raise e
                time.sleep(retry_delay)
        
        end_time = time.time()
        return [], None, end_time - start_time

    def answer_question(self, speaker_1_user_id, speaker_2_user_id, question, answer, category):
        """Answer a question using retrieved memories"""
        speaker_1_memories, _, speaker_1_memory_time = self.search_memory(speaker_1_user_id, question)
        speaker_2_memories, _, speaker_2_memory_time = self.search_memory(speaker_2_user_id, question)

        search_1_memory = [f"{item.get('timestamp', '')}: {item['memory']}" for item in speaker_1_memories]
        search_2_memory = [f"{item.get('timestamp', '')}: {item['memory']}" for item in speaker_2_memories]

        template = Template(self.ANSWER_PROMPT)
        answer_prompt = template.render(
            speaker_1_user_id=speaker_1_user_id.split("_")[0],
            speaker_2_user_id=speaker_2_user_id.split("_")[0],
            speaker_1_memories=json.dumps(search_1_memory, indent=4),
            speaker_2_memories=json.dumps(search_2_memory, indent=4),
            speaker_1_graph_memories=json.dumps([], indent=4),  # Cortex Mem doesn't have graph memories
            speaker_2_graph_memories=json.dumps([], indent=4),
            question=question,
        )

        t1 = time.time()
        response = self.openai_client.chat.completions.create(
            model=os.getenv("MODEL", "gpt-3.5-turbo"),
            messages=[{"role": "system", "content": answer_prompt}],
            temperature=0.0
        )
        t2 = time.time()
        response_time = t2 - t1
        
        return (
            response.choices[0].message.content,
            speaker_1_memories,
            speaker_2_memories,
            speaker_1_memory_time,
            speaker_2_memory_time,
            None,  # graph_memories
            None,
            response_time,
        )

    def process_question(self, val, speaker_a_user_id, speaker_b_user_id):
        """Process a single question"""
        question = val.get("question", "")
        answer = val.get("answer", "")
        category = val.get("category", -1)
        evidence = val.get("evidence", [])
        adversarial_answer = val.get("adversarial_answer", "")

        (
            response,
            speaker_1_memories,
            speaker_2_memories,
            speaker_1_memory_time,
            speaker_2_memory_time,
            speaker_1_graph_memories,
            speaker_2_graph_memories,
            response_time,
        ) = self.answer_question(speaker_a_user_id, speaker_b_user_id, question, answer, category)

        result = {
            "question": question,
            "answer": answer,
            "category": category,
            "evidence": evidence,
            "response": response,
            "adversarial_answer": adversarial_answer,
            "speaker_1_memories": speaker_1_memories,
            "speaker_2_memories": speaker_2_memories,
            "num_speaker_1_memories": len(speaker_1_memories),
            "num_speaker_2_memories": len(speaker_2_memories),
            "speaker_1_memory_time": speaker_1_memory_time,
            "speaker_2_memory_time": speaker_2_memory_time,
            "speaker_1_graph_memories": speaker_1_graph_memories,
            "speaker_2_graph_memories": speaker_2_graph_memories,
            "response_time": response_time,
        }

        # Save results after each question is processed
        with open(self.output_path, "w") as f:
            json.dump(self.results, f, indent=4)

        return result

    def process_data_file(self, file_path):
        """Process the entire data file"""
        with open(file_path, "r") as f:
            data = json.load(f)

        for idx, item in tqdm(enumerate(data), total=len(data), desc="Processing conversations"):
            qa = item["qa"]
            conversation = item["conversation"]
            speaker_a = conversation["speaker_a"]
            speaker_b = conversation["speaker_b"]

            speaker_a_user_id = f"{speaker_a}_{idx}"
            speaker_b_user_id = f"{speaker_b}_{idx}"

            for question_item in tqdm(
                qa, total=len(qa), desc=f"Processing questions for conversation {idx}", leave=False
            ):
                result = self.process_question(question_item, speaker_a_user_id, speaker_b_user_id)
                self.results[idx].append(result)

                # Save results after each question is processed
                with open(self.output_path, "w") as f:
                    json.dump(self.results, f, indent=4)

        # Final save at the end
        with open(self.output_path, "w") as f:
            json.dump(self.results, f, indent=4)
