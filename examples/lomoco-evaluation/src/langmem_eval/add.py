import json
import os
import time
import logging
import toml
from pathlib import Path
from typing import List, Dict, Any, Optional
from tqdm import tqdm

try:
    from qdrant_client import QdrantClient
    from qdrant_client.models import Distance, VectorParams, PointStruct, Filter, FieldCondition, MatchValue
except ImportError:
    raise ImportError(
        "qdrant-client is not installed. Please install it using: pip install qdrant-client"
    )

from .config_utils import check_openai_config, get_config_value, validate_config

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


class LangMemAdd:
    """Class to add memories to LangMem for evaluation using Qdrant vector database"""
    
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

        # Initialize LangMem components
        self._initialize_langmem()

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
        project_root = Path(__file__).parent.parent.parent.parent
        config_file = project_root / "config.toml"
        if config_file.exists():
            return str(config_file)

        raise FileNotFoundError("Could not find config.toml file")

    def _initialize_langmem(self):
        """Initialize LangMem with Qdrant vector database and embedding model"""
        try:
            # Load config
            config_data = toml.load(self.config_path)
            
            # Get Qdrant configuration
            qdrant_config = config_data.get("qdrant", {})
            self.qdrant_url = qdrant_config.get("url", "http://localhost:6334")
            self.collection_name = qdrant_config.get("collection_name", "memo-rs")
            
            # Get embedding configuration
            embedding_config = config_data.get("embedding", {})
            self.embedding_api_base_url = embedding_config.get("api_base_url", "")
            self.embedding_api_key = embedding_config.get("api_key", "")
            self.embedding_model_name = embedding_config.get("model_name", "")
            self.embedding_batch_size = embedding_config.get("batch_size", 10)
            
            # Initialize Qdrant client
            self.qdrant_client = QdrantClient(url=self.qdrant_url)
            
            # Create collection if it doesn't exist
            self._ensure_collection_exists()
            
            # Initialize embedding client
            import httpx
            from openai import OpenAI
            
            self.embedding_client = OpenAI(
                api_key=self.embedding_api_key,
                base_url=self.embedding_api_base_url,
                http_client=httpx.Client(verify=False)
            )
            
            # Get embedding dimension
            self.embedding_dim = self._get_embedding_dimension()
            
            logger.info(f"✅ LangMem initialized successfully with Qdrant at {self.qdrant_url}")
            logger.info(f"✅ Collection: {self.collection_name}, Embedding dim: {self.embedding_dim}")
            
        except Exception as e:
            logger.error(f"❌ Failed to initialize LangMem: {e}")
            raise

    def _ensure_collection_exists(self):
        """Ensure Qdrant collection exists, create if not"""
        try:
            collections = self.qdrant_client.get_collections().collections
            collection_names = [c.name for c in collections]
            
            if self.collection_name not in collection_names:
                # Get embedding dimension first
                embedding_dim = self._get_embedding_dimension()
                logger.info(f"Creating collection: {self.collection_name} with dim={embedding_dim}")
                self.qdrant_client.create_collection(
                    collection_name=self.collection_name,
                    vectors_config=VectorParams(size=embedding_dim, distance=Distance.COSINE)
                )
        except Exception as e:
            logger.warning(f"Could not ensure collection exists: {e}")

    def _get_embedding_dimension(self) -> int:
        """Get embedding dimension by making a test call"""
        try:
            response = self.embedding_client.embeddings.create(
                model=self.embedding_model_name,
                input=["test"]
            )
            return len(response.data[0].embedding)
        except Exception as e:
            logger.warning(f"Could not get embedding dimension, using default 1024: {e}")
            return 1024

    def _get_embedding(self, text: str) -> List[float]:
        """Get embedding for text"""
        try:
            response = self.embedding_client.embeddings.create(
                model=self.embedding_model_name,
                input=text
            )
            return response.data[0].embedding
        except Exception as e:
            logger.error(f"Error getting embedding: {e}")
            return []

    def load_data(self):
        if not self.data_path:
            raise ValueError("data_path not set")
        with open(self.data_path, "r") as f:
            self.data = json.load(f)
        return self.data

    def add_memory(self, user_id: str, content: str, timestamp: str = "") -> bool:
        """Add a memory using Qdrant vector database with embedding"""
        try:
            # Generate embedding for the content
            embedding = self._get_embedding(content)
            
            if not embedding:
                logger.error(f"❌ Failed to generate embedding for user {user_id}")
                self.stats["failed_memories"] += 1
                return False
            
            # Generate a unique ID for this memory
            import uuid
            memory_id = str(uuid.uuid4())
            
            # Create point for Qdrant
            point = PointStruct(
                id=memory_id,
                vector=embedding,
                payload={
                    "user_id": user_id,
                    "content": content,
                    "timestamp": timestamp,
                    "created_at": time.time()
                }
            )
            
            # Insert into Qdrant
            self.qdrant_client.upsert(
                collection_name=self.collection_name,
                points=[point]
            )
            
            self.stats["successful_memories"] += 1
            logger.debug(f"✅ Successfully added memory for user {user_id}")
            return True
            
        except Exception as e:
            logger.error(f"❌ Failed to add memory for user {user_id}: {e}")
            self.stats["failed_memories"] += 1
            return False

    def add_memories_for_speaker(self, speaker: str, messages: List[Dict], timestamp: str, desc: str):
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
                timestamp,
            )

            self.stats["total_memories"] += 1
            
            # Small delay between batches to avoid rate limiting
            time.sleep(0.3)
        
        if failed_batches > 0:
            logger.warning(f"{failed_batches}/{total_batches} batches failed for {speaker}")

    def process_conversation(self, item: Dict[str, Any], idx: int):
        """Process a single conversation with error handling"""
        try:
            conversation = item.get("conversation", {})
            speaker_a = conversation.get("speaker_a", "SpeakerA")
            speaker_b = conversation.get("speaker_b", "SpeakerB")

            speaker_a_user_id = f"{speaker_a}_{idx}"
            speaker_b_user_id = f"{speaker_b}_{idx}"

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