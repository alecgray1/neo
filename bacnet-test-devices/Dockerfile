FROM python:3.11-slim

WORKDIR /app

# Install bacpypes3 and ifaddr (required for network interface detection)
RUN pip install --no-cache-dir bacpypes3 ifaddr

# Copy virtual device script
COPY simple_device.py ./

# Make it executable
RUN chmod +x simple_device.py

# Expose BACnet/IP port
EXPOSE 47808/udp

# Default command (will be overridden in docker-compose)
CMD ["python3", "simple_device.py"]
