# Copyright 2026 Exochain Foundation
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at:
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#
# SPDX-License-Identifier: Apache-2.0

import os

from langchain.graphs.neo4j_graph import Neo4jGraph


def required_neo4j_env(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise RuntimeError(f"Missing required Neo4j configuration: {name}")
    return value


def build_graph() -> Neo4jGraph:
    return Neo4jGraph(
        url=required_neo4j_env("NEO4J_URI"),
        username=required_neo4j_env("NEO4J_USERNAME"),
        password=required_neo4j_env("NEO4J_PASSWORD"),
    )


graph = build_graph()

# Example Cypher Model Definitions:
# :Interaction - {id, text, timestamp, model}
# :MemoryObject - {embedding, tags}
# :Session - {id, started_at, archetype}

# Relationships:
# (:Interaction)-[:EMBEDDED_IN]->(:MemoryObject)
# (:Interaction)-[:EMANATES_FROM]->(:Session)
# (:MemoryObject)-[:TRACES_BACK_TO]->(:UPK)
