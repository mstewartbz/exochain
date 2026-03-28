"""Vector memory interface.

NOTE: External services (Weaviate) are optional for the MVP. The key point for
"gravity transfer" is that *writes* can be gated by decision.forum policy.
"""

from langchain.vectorstores import Weaviate
from langchain.embeddings import OpenAIEmbeddings

from utils import load_upk

def get_memory_vector_store():
    return Weaviate.from_texts(
        texts=[],  # Will fill dynamically
        embedding=OpenAIEmbeddings(),
        weaviate_url="http://localhost:8080"
    )

def store_memory(text_chunk, tags, *, decision_id: str | None = None):
    upk = load_upk()
    policy = (upk.get("governance") or {}).get("decision_forum") or {}
    required_for = set(policy.get("required_for") or [])
    if policy.get("enabled") and "memory_write" in required_for and not decision_id:
        raise ValueError(
            "decision.forum policy requires decision_id for memory writes. "
            "Create a Decision Record first and pass its id."
        )

    vs = get_memory_vector_store()
    vs.add_texts([text_chunk], metadatas=[{"tags": tags, "decision_id": decision_id}])
