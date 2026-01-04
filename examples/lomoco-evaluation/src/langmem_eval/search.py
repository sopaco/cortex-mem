import json
import logging
import os
import time
from collections import defaultdict
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

import toml
from jinja2 import Template
from openai import OpenAI
from tqdm import tqdm

try:
    from qdrant_client import QdrantClient
    from qdrant_client.models import FieldCondition, Filter, MatchValue
except ImportError:
    raise ImportError(
        "qdrant-client is not installed. Please install it using: pip install qdrant-client"
    )

from .config_utils import check_openai_config, get_config_value, validate_config
from .rate_limiter import RateLimiter

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


class LangMemSearch:
    """Class to search memories in LangMem for evaluation using Qdrant vector database"""

    def __init__(self, output_path="results.json", top_k=10, config_path=None):
        self.top_k = top_k
        self.results = defaultdict(list)
        self.output_path = output_path
        self.config_path = config_path or self._find_config_file()

        # Initialize rate limiters (30 calls per minute for each service)
        self.embedding_rate_limiter = RateLimiter(max_calls_per_minute=30)
        self.llm_rate_limiter = RateLimiter(max_calls_per_minute=30)

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

        # Initialize components
        self._initialize_components()

    def _initialize_components(self):
        """Initialize Qdrant client, embedding client, and LLM client"""
        try:
            # Load config
            config_data = toml.load(self.config_path)

            # Get Qdrant configuration
            qdrant_config = config_data.get("qdrant", {})
            qdrant_url = qdrant_config.get("url", "http://localhost:6334")
            self.collection_name = qdrant_config.get("collection_name", "memo-rs")

            # Parse URL to extract host and port for gRPC
            from urllib.parse import urlparse
            parsed_url = urlparse(qdrant_url)
            host = parsed_url.hostname or "localhost"
            port = parsed_url.port or 6334

            # Get embedding configuration
            embedding_config = config_data.get("embedding", {})
            self.embedding_api_base_url = embedding_config.get("api_base_url", "")
            self.embedding_api_key = embedding_config.get("api_key", "")
            self.embedding_model_name = embedding_config.get("model_name", "")

            # Get LLM configuration
            api_key = get_config_value(self.config_path, "llm", "api_key")
            api_base = get_config_value(self.config_path, "llm", "api_base_url")
            self.llm_model = get_config_value(
                self.config_path, "llm", "model_efficient", "gpt-3.5-turbo"
            )

            # Initialize Qdrant client with gRPC
            # Use prefer_grpc=True to force gRPC protocol
            self.qdrant_client = QdrantClient(
                host=host,
                grpc_port=port,
                prefer_grpc=True,
                timeout=qdrant_config.get("timeout_secs", 30)
            )

            # Initialize embedding client
            import httpx

            self.embedding_client = OpenAI(
                api_key=self.embedding_api_key,
                base_url=self.embedding_api_base_url,
                http_client=httpx.Client(verify=False),
            )

            # Initialize LLM client
            self.openai_client = OpenAI(
                api_key=api_key,
                base_url=api_base,
                http_client=httpx.Client(verify=False),
            )

            logger.info(
                f"✅ LangMemSearch initialized successfully with Qdrant at {host}:{port} (gRPC)"
            )
            logger.info(f"✅ Collection: {self.collection_name}")

        except Exception as e:
            logger.error(f"❌ Failed to initialize LangMemSearch: {e}")
            raise

    def _get_embedding(self, text: str) -> List[float]:
        """Get embedding for text"""
        try:
            with self.embedding_rate_limiter:
                response = self.embedding_client.embeddings.create(
                    model=self.embedding_model_name, input=text
                )
            return response.data[0].embedding
        except Exception as e:
            logger.error(f"Error getting embedding: {e}")
            return []

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

    def search_memory(
        self, user_id: str, query: str, max_retries: int = 3, retry_delay: float = 1
    ) -> Tuple[List[Dict], float]:
        """Search for memories using Qdrant vector database with semantic search"""
        start_time = time.time()
        retries = 0

        while retries < max_retries:
            try:
                # Generate embedding for the query
                query_embedding = self._get_embedding(query)

                if not query_embedding:
                    logger.error(f"❌ Failed to generate embedding for query: {query}")
                    return [], time.time() - start_time

                # Search in Qdrant with filter for user_id
                search_filter = Filter(
                    must=[
                        FieldCondition(key="user_id", match=MatchValue(value=user_id))
                    ]
                )

                # Use query_points instead of search (newer Qdrant API)
                search_results = self.qdrant_client.query_points(
                    collection_name=self.collection_name,
                    query=query_embedding,
                    query_filter=search_filter,
                    limit=self.top_k,
                    with_payload=True,
                ).points

                # Convert Qdrant results to memory format
                memories = []
                for result in search_results:
                    payload = result.payload
                    memories.append(
                        {
                            "memory": payload.get("content", ""),
                            "timestamp": payload.get("timestamp", ""),
                            "score": result.score,
                        }
                    )

                end_time = time.time()
                return memories, end_time - start_time

            except Exception as e:
                logger.error(f"Search error: {e}, retrying...")
                retries += 1
                if retries >= max_retries:
                    raise e
                time.sleep(retry_delay)

        end_time = time.time()
        return [], end_time - start_time

    def answer_question(
        self,
        speaker_1_user_id: str,
        speaker_2_user_id: str,
        question: str,
        answer: str,
        category: str,
    ) -> Tuple[str, List[Dict], List[Dict], float, float, None, None, float]:
        """Answer a question using retrieved memories"""
        # Sequential search (rate limiter handles the delays)
        speaker_1_memories, speaker_1_memory_time = self.search_memory(
            speaker_1_user_id, question
        )

        speaker_2_memories, speaker_2_memory_time = self.search_memory(
            speaker_2_user_id, question
        )

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
        with self.llm_rate_limiter:
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

    def process_question(
        self, val: Dict[str, Any], speaker_a_user_id: str, speaker_b_user_id: str
    ) -> Dict[str, Any]:
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
