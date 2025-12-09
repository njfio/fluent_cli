"""
Fluent CLI Agent Adapter for Terminal-Bench

This module implements the AbstractInstalledAgent interface to run the Fluent CLI
agent within Terminal-Bench's evaluation harness.

Usage:
    tb run --agent-import-path tbench_adapter.fluent_agent:FluentAgent -d terminal-bench-core
"""

import os
from pathlib import Path
from typing import Optional

# Terminal-bench imports - these must be available when running with tb
from terminal_bench.agents.installed_agents.abstract_installed_agent import (
    AbstractInstalledAgent,
)
from terminal_bench.terminal.models import TerminalCommand


class FluentAgent(AbstractInstalledAgent):
    """
    Fluent CLI Agent adapter for Terminal-Bench.

    This agent uses the Fluent CLI's agentic mode to solve terminal-bench tasks.
    It supports configurable models and iteration limits.

    Environment Variables:
        ANTHROPIC_API_KEY: Required for Anthropic models
        OPENAI_API_KEY: Required for OpenAI models
        FLUENT_MODEL: Override the default model (optional)
        FLUENT_MAX_ITERATIONS: Override max iterations (default: 50)
    """

    def __init__(
        self,
        model: Optional[str] = None,
        max_iterations: int = 50,
        enable_reflection: bool = False,
        **kwargs
    ):
        """
        Initialize the Fluent agent.

        Args:
            model: Model to use (e.g., 'claude-3-5-sonnet-20241022', 'gpt-4o')
            max_iterations: Maximum number of agent iterations
            enable_reflection: Whether to enable reflection mode
        """
        super().__init__(**kwargs)
        self._model = model or os.environ.get("FLUENT_MODEL", "claude-sonnet-4-20250514")
        self._max_iterations = max_iterations
        self._enable_reflection = enable_reflection

    @staticmethod
    def name() -> str:
        """Return the agent name for display and identification."""
        return "fluent"

    @property
    def _env(self) -> dict[str, str]:
        """
        Environment variables to pass to the agent container.

        Returns:
            Dictionary of environment variables including API keys and config.
        """
        env = {}

        # Pass through API keys if available
        if "ANTHROPIC_API_KEY" in os.environ:
            env["ANTHROPIC_API_KEY"] = os.environ["ANTHROPIC_API_KEY"]

        if "OPENAI_API_KEY" in os.environ:
            env["OPENAI_API_KEY"] = os.environ["OPENAI_API_KEY"]

        if "GOOGLE_API_KEY" in os.environ:
            env["GOOGLE_API_KEY"] = os.environ["GOOGLE_API_KEY"]

        # Fluent-specific configuration
        env["FLUENT_LOG_FORMAT"] = "human"
        env["FLUENT_VERBOSE"] = "1"

        # Allow commands needed for terminal-bench tasks
        env["FLUENT_ALLOW_COMMANDS"] = "git,cargo,npm,node,python,python3,pip,make,cmake,gcc,g++,rustc,go,java,javac,mvn,gradle,docker,kubectl,curl,wget,cat,ls,cd,mkdir,rm,cp,mv,touch,chmod,find,grep,sed,awk,head,tail,sort,uniq,wc,diff,patch,tar,gzip,gunzip,zip,unzip,ssh,scp,rsync"

        return env

    @property
    def _install_agent_script_path(self) -> os.PathLike:
        """
        Path to the shell script that installs the Fluent agent.

        Returns:
            Path to install_fluent.sh script.
        """
        # Get the directory containing this module
        module_dir = Path(__file__).parent
        return module_dir / "install_fluent.sh"

    def _run_agent_commands(self, task_description: str) -> list[TerminalCommand]:
        """
        Generate commands to run the Fluent agent on a task.

        Args:
            task_description: The task description from terminal-bench.

        Returns:
            List of TerminalCommand objects to execute.
        """
        # Escape the task description for shell
        escaped_task = task_description.replace("'", "'\\''")

        # First, update the config file with the actual API key
        # This is needed because the install script runs before env vars are fully set
        config_setup_cmd = '''sed -i "s/bearer_token = .*/bearer_token = \\"$ANTHROPIC_API_KEY\\"/" /app/fluent_config.toml'''

        # Build the fluent command (use absolute path since /app isn't in PATH)
        cmd_parts = [
            "/app/fluent", "agent",
            "--agentic",
            "--goal", f"'{escaped_task}'",
            "--max-iterations", str(self._max_iterations),
            "--model", self._model,
            "--enable-tools",
            "--agent-config", "/app/agent_config.json",
            "--config", "/app/fluent_config.toml",
        ]

        if self._enable_reflection:
            cmd_parts.append("--reflection")

        fluent_command = " ".join(cmd_parts)

        # Combine config setup and fluent command
        full_command = f"{config_setup_cmd} && {fluent_command}"

        # Set a generous timeout (30 minutes per task by default)
        timeout_sec = 1800.0

        return [
            TerminalCommand(
                command=full_command,
                timeout_sec=timeout_sec,
            )
        ]


class FluentAgentReflection(FluentAgent):
    """Fluent agent with reflection mode enabled."""

    def __init__(self, **kwargs):
        kwargs["enable_reflection"] = True
        super().__init__(**kwargs)

    @staticmethod
    def name() -> str:
        return "fluent-reflection"


class FluentAgentFast(FluentAgent):
    """Fluent agent configured for faster iteration (fewer max iterations)."""

    def __init__(self, **kwargs):
        kwargs.setdefault("max_iterations", 20)
        super().__init__(**kwargs)

    @staticmethod
    def name() -> str:
        return "fluent-fast"


# For testing the module directly
if __name__ == "__main__":
    agent = FluentAgent()
    print(f"Agent name: {agent.name()}")
    print(f"Install script: {agent._install_agent_script_path}")
    print(f"Environment: {agent._env}")

    test_task = "Write a Python script that prints 'Hello, World!'"
    commands = agent._run_agent_commands(test_task)
    for cmd in commands:
        print(f"Command: {cmd.command}")
        print(f"Timeout: {cmd.timeout_sec}s")
