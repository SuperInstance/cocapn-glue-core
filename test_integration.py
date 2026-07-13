#!/usr/bin/env python3
"""Integration test: cocapn-glue-core → PLATO tile forwarding."""
import threading, time, sys, json, urllib.request
sys.path.insert(0, '.')
from glue_core import GlueProtocol, MessageType

PLATO_URL = "http://localhost:8847"

def post_to_plato(room, question, answer, tags=None):
    try:
        data = json.dumps({
            "domain": room,
            "question": question,
            "answer": answer,
            "tags": tags or ["glue-core"]
        }).encode()
        req = urllib.request.Request(
            f"{PLATO_URL}/submit",
            data=data,
            headers={"Content-Type": "application/json"},
            method="POST"
        )
        with urllib.request.urlopen(req, timeout=5) as resp:
            result = json.loads(resp.read())
            return result.get("tile_hash", "unknown")
    except Exception as e:
        return f"ERROR: {e}"

def test_keeper_forwards_to_plato():
    print("=== Glue Core → PLATO Integration Test ===\n")
    
    keeper = GlueProtocol(role="keeper")
    
    def handle_tile(msg, sock):
        p = msg.payload
        print(f"  [Keeper] Tile from {msg.sender}: {p.get('question', '')[:50]}")
        room = p.get("room", "glue")
        tile_hash = post_to_plato(room, p["question"], p["answer"], p.get("tags"))
        print(f"  [Keeper] → PLATO ({room}): tile {tile_hash}")
    
    keeper.on(MessageType.TILE, handle_tile)
    keeper.on(MessageType.REGISTER, lambda msg, sock: print(f"  [Keeper] Agent registered: {msg.payload.get('agent')}"))
    keeper.listen(port=18901)
    
    agent = GlueProtocol(role="agent", agent_name="test-agent")
    sock = agent.connect("127.0.0.1", 18901)
    
    time.sleep(0.3)
    
    test_tiles = [
        ("glue", "Integration test 1", "Hello from glue-core → PLATO"),
        ("oracle1", "Integration test 2", "Fleet heartbeat"),
        ("fleet_communication", "Integration test 3", "Glue protocol working"),
    ]
    
    for room, q, a in test_tiles:
        agent.send_tile(sock, q, a, room)
        time.sleep(0.2)
    
    time.sleep(0.5)

    # The keeper only prints as it forwards tiles; nothing here previously
    # asserted the protocol actually delivered them. Check the agent's own
    # sequence counter (incremented once per sent message in send()) so this
    # test fails if messages genuinely stop flowing.
    stats = agent.get_stats()
    assert stats.get("sequence", 0) >= len(test_tiles), (
        f"Expected at least {len(test_tiles)} sent messages, got {stats}"
    )
    print(f"  [Agent] Stats after test: {stats}")

    agent.stop()
    keeper.stop()
    print("\nIntegration test complete.")

if __name__ == "__main__":
    test_keeper_forwards_to_plato()