"""
Simple RAG Baseline for Comparison

This module implements a simple RAG (Retrieval-Augmented Generation) baseline
to compare against Cortex Mem's performance. Uses:
- Simple vector similarity for retrieval
- Basic chunking of conversation text
- Same LLM for answer generation
"""

import json
import time
import os
from pathlib import Path
from collections import defaultdict
from typing import List, Dict, Tuple
from tqdm import tqdm
from jinja2 import Template

import httpx
from openai import OpenAI
from sentence_transformers import SentenceTransformer
import numpy as np


class SimpleRAGBaseline:
    """Simple RAG baseline for comparison with Cortex Mem"""
    
    def __init__(self, config_path: str):
        """Initialize Simple RAG baseline"""
        import toml
        
        # Load config
        config = toml.load(config_path)
        
        # Initialize LLM client
        self.llm_client = OpenAI(
            api_key=config['llm']['api_key'],
            base_url=config['llm']['api_base_url'],
            http_client=httpx.Client(verify=False)
        )
        self.llm_model = config['llm'].get('model_efficient', 'gpt-3.5-turbo')
        
        # Initialize embedding model
        self.embedding_model = SentenceTransformer(config['embedding'].get('model_name', 'all-MiniLM-L6-v2'))
        
        # Initialize embedding API client
        self.embedding_client = OpenAI(
            api_key=config['embedding'].get('api_key', ''),
            base_url=config['embedding'].get('api_base_url', ''),
            http_client=httpx.Client(verify=False)
        )
        self.embedding_model_name = config['embedding'].get('model_name', '')
        
        # Storage for memories
        self.memories = {}
        
    def get_embedding(self, text: str) -> np.ndarray:
        """Get embedding for text"""
        try:
            response = self.embedding_client.embeddings.create(
                model=self.embedding_model_name,
                input=text
            )
            return np.array(response.data[0].embedding, dtype=np.float32)
        except Exception as e:
            print(f"Error getting embedding: {e}")
            # Fallback to local model
            embedding = self.embedding_model.encode(text)
            return np.array(embedding, dtype=np.float32)
    
    def add_memories_from_conversation(self, conversation_data: dict, conv_id: int):
        """
        Extract and store memories from conversation
        
        Args:
            conversation_data: Conversation dictionary
            conv_id: Conversation ID
        """
        speaker_a = conversation_data.get("speaker_a", "SpeakerA")
        speaker_b = conversation_data.get("speaker_b", "SpeakerB")
        
        speaker_a_memories = []
        speaker_b_memories = []
        
        # Extract messages from all sessions
        for key in conversation_data.keys():
            if key in ["speaker_a", "speaker_b"] or "date" in key or "timestamp" in key:
                continue
            
            date_time_key = key + "_date_time"
            timestamp = conversation_data.get(date_time_key, "")
            chats = conversation_data[key]
            
            for chat in chats:
                speaker = chat.get("speaker", "")
                text = chat.get("text", "")
                
                memory_text = f"{speaker}: {text}"
                memory_entry = {
                    "content": memory_text,
                    "timestamp": timestamp,
                    "speaker": speaker
                }
                
                if speaker == speaker_a:
                    speaker_a_memories.append(memory_entry)
                elif speaker == speaker_b:
                    speaker_b_memories.append(memory_entry)
        
        # Store memories with embeddings
        self.memories[f"{speaker_a}_{conv_id}"] = self._index_memories(speaker_a_memories)
        self.memories[f"{speaker_b}_{conv_id}"] = self._index_memories(speaker_b_memories)
    
    def _index_memories(self, memories: List[Dict]) -> Dict:
        """Index memories with embeddings"""
        indexed = {
            "memories": memories,
            "embeddings": []
        }
        
        for memory in memories:
            embedding = self.get_embedding(memory["content"])
            indexed["embeddings"].append(embedding)
        
        return indexed
    
    def search_memories(self, user_id: str, query: str, top_k: int = 10) -> Tuple[List[Dict], float]:
        """
        Search for relevant memories using vector similarity
        
        Args:
            user_id: User identifier
            query: Search query
            top_k: Number of results to return
            
        Returns:
            Tuple of (retrieved memories, search time)
        """
        start_time = time.time()
        
        if user_id not in self.memories:
            return [], time.time() - start_time
        
        indexed_data = self.memories[user_id]
        memories = indexed_data["memories"]
        embeddings = np.array(indexed_data["embeddings"])
        
        if len(embeddings) == 0:
            return [], time.time() - start_time
        
        # Get query embedding
        query_embedding = self.get_embedding(query)
        
        # Calculate similarities
        similarities = np.dot(embeddings, query_embedding)
        
        # Get top k indices
        top_k_indices = np.argsort(similarities)[-top_k:][::-1]
        
        # Retrieve memories
        retrieved = []
        for idx in top_k_indices:
            memory = memories[idx].copy()
            memory["score"] = float(similarities[idx])
            retrieved.append(memory)
        
        search_time = time.time() - start_time
        return retrieved, search_time
    
    def answer_question(self, speaker_1_user_id: str, speaker_2_user_id: str,
                      question: str, top_k: int = 10) -> Dict:
        """
        Answer a question using retrieved memories
        
        Args:
            speaker_1_user_id: First speaker's user ID
            speaker_2_user_id: Second speaker's user ID
            question: The question to answer
            top_k: Number of memories to retrieve
            
        Returns:
            Dictionary with answer and metadata
        """
        # Search memories for both speakers
        speaker_1_memories, s1_time = self.search_memories(speaker_1_user_id, question, top_k)
        time.sleep(0.5)  # Small delay
        speaker_2_memories, s2_time = self.search_memories(speaker_2_user_id, question, top_k)
        
        # Prepare prompt
        template = Template(self._get_answer_prompt())
        prompt = template.render(
            speaker_1_user_id=speaker_1_user_id.split("_")[0],
            speaker_2_user_id=speaker_2_user_id.split("_")[0],
            speaker_1_memories=json.dumps([m["content"] for m in speaker_1_memories], indent=2),
            speaker_2_memories=json.dumps([m["content"] for m in speaker_2_memories], indent=2),
            question=question
        )
        
        # Generate answer
        start_time = time.time()
        response = self.llm_client.chat.completions.create(
            model=self.llm_model,
            messages=[{"role": "system", "content": prompt}],
            temperature=0.0,
        )
        response_time = time.time() - start_time
        
        return {
            "response": response.choices[0].message.content,
            "speaker_1_memories": speaker_1_memories,
            "speaker_2_memories": speaker_2_memories,
            "speaker_1_memory_time": s1_time,
            "speaker_2_memory_time": s2_time,
            "response_time": response_time,
            "num_speaker_1_memories": len(speaker_1_memories),
            "num_speaker_2_memories": len(speaker_2_memories),
        }
    
    def _get_answer_prompt(self) -> str:
        """Get answer generation prompt"""
        return """
You are an intelligent memory assistant. Your task is to answer questions based on retrieved conversation memories.

# CONTEXT
You have access to memories from two speakers in a conversation. These memories contain information that may be relevant to answering the question.

# INSTRUCTIONS
1. Carefully analyze all provided memories from both speakers
2. Look for explicit mentions of the answer in the memories
3. Focus only on the content of the memories
4. Provide a concise answer (less than 5-6 words)
5. If the memories don't contain the answer, say "I don't have enough information"

# MEMORIES

Memories for {{speaker_1_user_id}}:
{{speaker_1_memories}}

Memories for {{speaker_2_user_id}}:
{{speaker_2_memories}}

# QUESTION

Question: {{question}}

Answer:
"""


class SimpleRAGEvaluator:
    """Evaluator for Simple RAG baseline"""
    
    def __init__(self, data_path: str, config_path: str):
        """Initialize evaluator"""
        self.rag = SimpleRAGBaseline(config_path)
        self.data_path = data_path
        self.results = defaultdict(list)
    
    def load_data(self):
        """Load dataset"""
        with open(self.data_path, 'r') as f:
            self.data = json.load(f)
    
    def process_all(self, output_path: str, top_k: int = 10):
        """
        Process all conversations and generate answers
        
        Args:
            output_path: Path to save results
            top_k: Number of memories to retrieve
        """
        print("🔄 Loading dataset...")
        self.load_data()
        
        print("🔄 Indexing conversations...")
        for idx, item in enumerate(tqdm(self.data, desc="Indexing")):
            self.rag.add_memories_from_conversation(item["conversation"], idx)
        
        print("🔄 Answering questions...")
        for idx, item in enumerate(tqdm(self.data, desc="Answering")):
            conversation = item["conversation"]
            speaker_a = conversation.get("speaker_a", "SpeakerA")
            speaker_b = conversation.get("speaker_b", "SpeakerB")
            
            speaker_a_user_id = f"{speaker_a}_{idx}"
            speaker_b_user_id = f"{speaker_b}_{idx}"
            
            for qa_item in item["qa"]:
                question = qa_item["question"]
                answer = qa_item["answer"]
                category = qa_item["category"]
                evidence = qa_item.get("evidence", [])
                adversarial_answer = qa_item.get("adversarial_answer", "")
                
                result = self.rag.answer_question(
                    speaker_a_user_id,
                    speaker_b_user_id,
                    question,
                    top_k
                )
                
                result.update({
                    "question": question,
                    "answer": answer,
                    "category": category,
                    "evidence": evidence,
                    "adversarial_answer": adversarial_answer,
                })
                
                self.results[idx].append(result)
                
                # Save after each question
                with open(output_path, 'w') as f:
                    json.dump(self.results, f, indent=2)
        
        print(f"\n✅ Evaluation complete! Results saved to {output_path}")


def run_baseline_evaluation(data_path: str, config_path: str = "config.toml",
                         output_path: str = "results/simple_rag_results.json"):
    """Run Simple RAG baseline evaluation"""
    evaluator = SimpleRAGEvaluator(data_path, config_path)
    evaluator.process_all(output_path)


if __name__ == "__main__":
    import argparse
    
    parser = argparse.ArgumentParser(description="Run Simple RAG baseline")
    parser.add_argument("--data", required=True, help="Path to dataset file")
    parser.add_argument("--config", default="config.toml", help="Path to config file")
    parser.add_argument("--output", default="results/simple_rag_results.json", help="Output path")
    parser.add_argument("--top_k", type=int, default=10, help="Number of memories to retrieve")
    
    args = parser.parse_args()
    
    run_baseline_evaluation(args.data, args.config, args.output)
