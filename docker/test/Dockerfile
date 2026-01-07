# WADUP Test Runner Image
# Executes WASM modules against sample files using wasmtime

FROM python:3.12-slim-bookworm

# Install wasmtime Python package
RUN pip install --no-cache-dir wasmtime==27.0.0

# Create runner user
RUN useradd -m -s /bin/bash runner

# Create directories
RUN mkdir -p /test/module /test/sample /test/output && \
    chown -R runner:runner /test

# Copy test runner script
# Built with: docker build -f docker/test/Dockerfile .
COPY --chown=runner:runner docker/test/test_runner.py /usr/local/bin/test_runner.py
RUN chmod +x /usr/local/bin/test_runner.py

USER runner
WORKDIR /test

# Default entrypoint runs the test runner
ENTRYPOINT ["python3", "/usr/local/bin/test_runner.py"]
