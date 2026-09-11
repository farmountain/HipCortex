from setuptools import setup, find_packages

setup(
    name="hipcortex",
    version="3.10.0",
    description="Cognitive state substrate for AI agents — Python SDK (universal server-side passive capture, WAL-persistent, MCP + REST)",
    long_description=open("../../README.md").read() if False else "",
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
