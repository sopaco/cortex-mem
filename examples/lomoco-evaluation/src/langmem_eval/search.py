import json
import os
import time
import logging
from collections import defaultdict
from pathlib import Path
from typing import List, Dict, Tuple, Any

from jinja2 import Template
from openai import OpenAI
from tqdm import tqdm

try:
    from langgraph.store.memory import InMemoryStore
except ImportError:
    raise ImportError(
        "langgraph is not installed. Please install it using: pip install langgraph"
    )

from .config_utils import check_openai_config, get_config_value, validate_config

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


class LangMemSearch:
    """Class to search memories in LangMem for evaluation"""
    
    def __init__(self, output_path="results.json", top_k=10, config_path=None):
        self.top_k = top_k
        self.results = defaultdict(list)
        self.output_path = output_path
        self.config_path = config_path or self._find_config_file()
        
        # Answer generation prompt (same as Cortex Mem)
        self.ANSWER_PROMPT = """
You are an intelligent memory assistant tasked with retrieving accurate information from conversation memories.

# CONTEXT:
You have access to memories from two speakers in a conversation. These memories contain 
timestamped information that may be relevant to answering the question.

# INSTRUCTIONS:
1. Carefully analyze all provided memories from both speakers
2. Pay special attention to the timestamps to determine the answer
3. If the question asks about a specific event or fact, look for direct evidence in the 
   memories
4. If the memories contain contradictory information, prioritize the most recent memory
5. If there is a question about time references (like "last year", "two months ago", 
   etc.), calculate the actual date based on the memory timestamp. For example, if a 
   memory from 4 May 2022 mentions "went to India last year," then the trip occurred 
   in 2021.
6. Always convert relative time references to specific dates, months, or years. For 
   example, convert "last year" to "2022" or "two months ago" to "March 2023" based 
   on the memory timestamp. Ignore the reference while answering the question.
7. Focus only on the content of the memories from both speakers. Do not confuse 
   character names mentioned in memories with the actual users who created those 
   memories.
8. The answer should be less than 5-6 words.

# APPROACH (Think step by step):
1. First, examine all memories that contain information related to the question
2. Examine the timestamps and content of these memories carefully
3. Look for explicit mentions of dates, times, locations, or events that answer the 
   question
4. If the answer requires calculation (e.g., converting relative time references), 
   show your work
5. Formulate a precise, concise answer based solely on the evidence in the memories
6. Double-check that your answer directly addresses the question asked
7. Ensure your final answer is specific and avoids vague time references

Memories for user {{speaker_1_user_id}}:

{{speaker_1_memories}}

Memories for user {{speaker_2_user_id}}:

{{speaker_2_memories}}

Question: {{question}}

Answer:
"""
        
        # Validate config file
        if not validate_config(self.config_path):
            raise ValueError(f"Invalid config file: {self.config_path}")
        
        # Check OpenAI configuration
        if not check_openai_config(self.config_path):
            raise ValueError(
                f"OpenAI configuration not properly set in {self.config_path}"
            )
        
        # Initialize OpenAI client from config.toml
        api_key = get_config_value(self.config_path, "llm", "api_key")
        api_base = get_config_value(self.config_path, "llm", "api_base_url")
        self.llm_model = get_config_value(self.config_path, "llm", "model_efficient", "gpt-3.5-turbo")
        
        # Create HTTP client with SSL verification disabled for internal APIs
        import httpx
        http_client = httpx.Client(verify=False)
        
        self.openai_client = OpenAI(
            api_key=api_key,
            base_url=api_base,
            http_client=http_client
        )
        
        # Initialize LangMem store
        # Note: This will be a new store. For persistence, we need to use the same store instance
        # or use a persistent store. For now, we'll assume memories are added in the same session.
        self.store = InMemoryStore()
        
        # Try to load previously stored memories from a file if exists
        self._load_memories_from_file()
    
    def _load_memories_from_file(self):
        """Load memories from a JSON file if it exists"""
        import json
        memory_file = "results/langmem_store.json"
        try:
            if os.path.exists(memory_file):
                print(f"📂 Found memory file: {memory_file}")
                with open(memory_file, 'r') as f:
                    memories_dict = json.load(f)
                
                print(f"✅ Loaded JSON with {len(memories_dict)} namespaces")
                
                # Restore memories to store
                total_items = 0
                for namespace_str, items in memories_dict.items():
                    namespace_tuple = tuple(namespace_str.split('/'))
                    for key, item_data in items.items():
                        self.store.put(namespace_tuple, key, item_data["value"])
                        total_items += 1
                
                print(f"✅ Successfully loaded {total_items} memories from {memory_file}")
        except Exception as e:
            print(f"⚠️  Could not load memories from file: {e}")
            import traceback
            traceback.print_exc()
    
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
        project_root = Path(__file__).parent.parent.parent.parent
        config_file = project_root / "config.toml"
        if config_file.exists():
            return str(config_file)
        
        raise FileNotFoundError("Could not find config.toml file")
    
    def search_memory(self, user_id: str, query: str, max_retries: int = 3, retry_delay: float = 1) -> Tuple[List[Dict], float]:
        """Search for memories using LangMem store"""
        start_time = time.time()
        retries = 0
        
        while retries < max_retries:
            try:
                # Create namespace for this user
                namespace = ("memories", user_id)
                
                # Search memories in the store
                # LangMem store supports semantic search through the search method
                memories = []
                
                # Get all memories for this user
                all_memories = list(self.store.search(namespace))
                
                # Debug: print what we found
                if len(all_memories) == 0:
                    # Try to search with empty namespace to see all memories
                    all_items = list(self.store.search(()))
                    logger.debug(f"Total items in store: {len(all_items)}")
                    if len(all_items) > 0:
                        # Print first few items to see structure
                        for i, item in enumerate(all_items[:3]):
                            logger.debug(f"Item {i}: namespace={item.namespace}, key={item.key}")
                
                # Simple relevance scoring based on query matching
                # In a real implementation, you would use embedding-based similarity
                query_lower = query.lower()
                scored_memories = []
                
                for memory_item in all_memories:
                    memory_value = memory_item.value
                    
                    # Convert memory to string if it's not
                    if isinstance(memory_value, dict):
                        memory_content = str(memory_value)
                    else:
                        memory_content = str(memory_value)
                    
                    # Simple keyword matching score
                    score = 0.0
                    query_words = query_lower.split()
                    for word in query_words:
                        if word in memory_content.lower():
                            score += 1.0
                    
                    if score > 0:
                        scored_memories.append({
                            "memory": memory_content,
                            "timestamp": "",  # LangMem doesn't store timestamp by default
                            "score": score,
                        })
                
                # Sort by score and take top_k
                scored_memories.sort(key=lambda x: x["score"], reverse=True)
                memories = scored_memories[:self.top_k]
                
                end_time = time.time()
                return memories, end_time - start_time
            
            except Exception as e:
                print(f"Search error: {e}, retrying...")
                retries += 1
                if retries >= max_retries:
                    raise e
                time.sleep(retry_delay)
        
        end_time = time.time()
        return [], end_time - start_time
    
    def answer_question(
        self, speaker_1_user_id: str, speaker_2_user_id: str, 
        question: str, answer: str, category: str
    ) -> Tuple[str, List[Dict], List[Dict], float, float, None, None, float]:
        """Answer a question using retrieved memories"""
        # Sequential search to avoid rate limiting
        speaker_1_memories, speaker_1_memory_time = self.search_memory(
            speaker_1_user_id, question
        )
        # Add a small delay between searches to avoid rate limiting
        time.sleep(2)
        
        speaker_2_memories, speaker_2_memory_time = self.search_memory(
            speaker_2_user_id, question
        )
        # Add a small delay before LLM call
        time.sleep(2)
        
        search_1_memory = [
            f"{item.get('timestamp', '')}: {item['memory']}"
            for item in speaker_1_memories
        ]
        search_2_memory = [
            f"{item.get('timestamp', '')}: {item['memory']}"
            for item in speaker_2_memories
        ]
        
        template = Template(self.ANSWER_PROMPT)
        answer_prompt = template.render(
            speaker_1_user_id=speaker_1_user_id.split("_")[0],
            speaker_2_user_id=speaker_2_user_id.split("_")[0],
            speaker_1_memories=json.dumps(search_1_memory, indent=4),
            speaker_2_memories=json.dumps(search_2_memory, indent=4),
            question=question,
        )
        
        t1 = time.time()
        response = self.openai_client.chat.completions.create(
            model=self.llm_model,
            messages=[{"role": "system", "content": answer_prompt}],
            temperature=0.0,
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
    
    def process_question(self, val: Dict[str, Any], speaker_a_user_id: str, speaker_b_user_id: str) -> Dict[str, Any]:
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
        ) = self.answer_question(
            speaker_a_user_id, speaker_b_user_id, question, answer, category
        )
        
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
    
    def process_data_file(self, file_path: str):
        """Process the entire data file"""
        with open(file_path, "r") as f:
            data = json.load(f)
        
        for idx, item in tqdm(
            enumerate(data), total=len(data), desc="Processing conversations"
        ):
            qa = item["qa"]
            conversation = item["conversation"]
            speaker_a = conversation["speaker_a"]
            speaker_b = conversation["speaker_b"]
            
            speaker_a_user_id = f"{speaker_a}_{idx}"
            speaker_b_user_id = f"{speaker_b}_{idx}"
            
            for question_item in tqdm(
                qa,
                total=len(qa),
                desc=f"Processing questions for conversation {idx}",
                leave=False,
            ):
                result = self.process_question(
                    question_item, speaker_a_user_id, speaker_b_user_id
                )
                self.results[idx].append(result)
                
                # Save results after each question is processed
                with open(self.output_path, "w") as f:
                    json.dump(self.results, f, indent=4)
        
        # Final save at the end
        with open(self.output_path, "w") as f:
            json.dump(self.results, f, indent=4)