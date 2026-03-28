from langchain.graphs.neo4j_graph import Neo4jGraph

graph = Neo4jGraph(
    url="bolt://localhost:7687",
    username="neo4j",
    password="password"  # secure via env var in prod
)

# Example Cypher Model Definitions:
# :Interaction - {id, text, timestamp, model}
# :MemoryObject - {embedding, tags}
# :Session - {id, started_at, archetype}

# Relationships:
# (:Interaction)-[:EMBEDDED_IN]->(:MemoryObject)
# (:Interaction)-[:EMANATES_FROM]->(:Session)
# (:MemoryObject)-[:TRACES_BACK_TO]->(:UPK)
