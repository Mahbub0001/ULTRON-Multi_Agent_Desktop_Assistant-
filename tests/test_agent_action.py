import pytest
from actions.agent_town import delegate_agent_task, TOOL

def test_tool_declaration():
    assert TOOL["name"] == "delegate_agent_task"
    assert "parameters" in TOOL
    assert "agent" in TOOL["parameters"]["properties"]
    assert "task" in TOOL["parameters"]["properties"]

def test_delegate_agent_task_execution(monkeypatch):
    dispatched = []
    import core.agent_town as at
    mgr = at.AgentTownManager.get_instance()
    
    def mock_dispatch(agent_name, task, on_complete=None, async_exec=True):
        dispatched.append((agent_name, task))
        return True
        
    monkeypatch.setattr(mgr, "dispatch_task", mock_dispatch)
    
    spoken = []
    def mock_speak(text):
        spoken.append(text)
        
    res = delegate_agent_task(
        {"agent": "Bob", "task": "Write a python script"},
        speak=mock_speak
    )
    assert "Bob" in res
    assert len(dispatched) == 1
    assert dispatched[0][0] == "Bob"
    assert dispatched[0][1] == "Write a python script"
