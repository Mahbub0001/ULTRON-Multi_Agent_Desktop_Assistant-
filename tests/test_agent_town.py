import pytest
from core.agent_town import AgentTownManager, AgentState

def test_load_default_agents():
    mgr = AgentTownManager.get_instance()
    agents = mgr.get_all_agents()
    assert len(agents) == 4
    names = [a.name for a in agents]
    assert "Alice" in names
    assert "Bob" in names
    assert "Carol" in names
    assert "Dave" in names
    assert mgr.get_agent("Alice").state == AgentState.IDLE

def test_dispatch_task_sync_mock(monkeypatch):
    mgr = AgentTownManager.get_instance()
    
    # Mock LLM execution to avoid live API calls during unit test
    def mock_call(contents, tier="fast", timeout_ms=30000):
        return "Mock research report from agent."
        
    import core.gemini as gemini
    monkeypatch.setattr(gemini, "call", mock_call)
    
    finished = []
    def on_complete(agent, result):
        finished.append((agent.name, result))
        
    mgr.dispatch_task("Alice", "Find recent AI agent news", on_complete=on_complete, async_exec=False)
    
    alice = mgr.get_agent("Alice")
    assert alice.state == AgentState.COMPLETED
    assert alice.latest_result == "Mock research report from agent."
    assert len(finished) == 1
    assert finished[0][0] == "Alice"
