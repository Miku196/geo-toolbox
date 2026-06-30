from setuptools import setup, find_packages

setup(
    name="geo-toolbox-magic",
    version="0.1.0",
    description="Jupyter/IPython magic extension for geo-toolbox — browser-grade GIS in notebooks",
    long_description=open("README.md", encoding="utf-8").read(),
    long_description_content_type="text/markdown",
    author="geo-toolbox contributors",
    url="https://github.com/geo-toolbox/geo-toolbox",
    packages=find_packages(),
    python_requires=">=3.9",
    install_requires=[
        "geo-toolbox>=0.1.0",
        "ipython>=7.0",
    ],
    classifiers=[
        "Development Status :: 3 - Alpha",
        "Intended Audience :: Science/Research",
        "License :: OSI Approved :: MIT License",
        "Programming Language :: Python :: 3",
        "Topic :: Scientific/Engineering :: GIS",
        "Framework :: Jupyter",
        "Framework :: IPython",
    ],
)
