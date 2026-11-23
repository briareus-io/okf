"""Exercise labeled files and individual OKF commands from a Markdown task guide."""
import re
import shlex
from pathlib import Path


def run_guide(path, directory, run_command):
    """Write labeled fixtures and dispatch each published command in reading order.

    run_command receives the CLI arguments after "okf" and the example directory.
    Return (args, result) pairs so each guide can assert its own expected outcomes.
    """
    directory = Path(directory)
    directory.mkdir(parents=True, exist_ok=True)
    steps = re.compile(
        r"(?:^\*\*File: `([^`\n]+)`\*\*\n\s*\n)?"
        r"^```([^\n]*)\n(.*?)^```[ \t]*$",
        re.MULTILINE | re.DOTALL,
    )
    results = []
    for filename, language, content in steps.findall(Path(path).read_text()):
        if filename:
            relative = Path(filename)
            assert not relative.is_absolute() and ".." not in relative.parts, filename
            destination = directory / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_text(content)
        elif language == "sh":
            # Only literal CLI invocations are executable. Prose and output stay inert.
            command = content.strip().replace("\\\n", " ")
            assert "\n" not in command, f"{path}: expected one command per shell block"
            words = shlex.split(command)
            assert words and words[0] == "okf", f"{path}: expected an okf command"
            args = words[1:]
            results.append((args, run_command(args, directory)))
    return results
