from setuptools import setup, find_packages

setup(
    name="hipcortex",
    version="3.11.0",
    description="Cognitive state substrate for AI agents — Python SDK + CLI (universal server-side passive capture, 61-tool / 7-resource MCP server, WAL-persistent, MCP + REST)",
    # No long_description here: PEP 621 metadata in pyproject.toml supplies readme = "README.md",
    # and declaring it in both places is a duplicate-metadata error. This used to be a dead
    # expression (open(...).read() if False else "") that read as if it did something.
    author="HipCortex Contributors",
    license="Apache-2.0",
    packages=find_packages(),
    python_requires=">=3.9",
    install_requires=[
        "requests>=2.28",
    ],
    extras_require={
        "langchain": ["langchain>=0.1"],
        "llamaindex": ["llama-index-core>=0.10"],
        "crewai": ["crewai>=0.28", "crewai-tools>=0.1"],
        "autogen": ["pyautogen>=0.2"],
        "benchmark": ["mem0ai>=0.1", "tabulate>=0.9"],
        "all": [
            "langchain>=0.1",
            "llama-index-core>=0.10",
            "crewai>=0.28",
            "crewai-tools>=0.1",
            "pyautogen>=0.2",
            "mem0ai>=0.1",
            "tabulate>=0.9",
        ],
    },
    classifiers=[
        "Programming Language :: Python :: 3",
        "License :: OSI Approved :: Apache Software License",
    ],
)
