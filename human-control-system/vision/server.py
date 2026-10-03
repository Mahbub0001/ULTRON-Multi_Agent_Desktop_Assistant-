#!/usr/bin/env python3
"""Vision Service gRPC Server Entry Point."""

import argparse
import logging
import signal
import sys
import time
from pathlib import Path

import structlog

from src.config import load_config, VisionConfig
from src.service.vision_service import serve, VisionService


def setup_logging(config: VisionConfig):
    """Configure structured logging."""
    log_level = getattr(logging, config.logging.level.upper(), logging.INFO)

    if config.logging.format == "json":
        structlog.configure(
            processors=[
                structlog.stdlib.filter_by_level,
                structlog.stdlib.add_logger_name,
                structlog.stdlib.add_log_level,
                structlog.stdlib.PositionalArgumentsFormatter(),
                structlog.processors.TimeStamper(fmt="iso"),
                structlog.processors.StackInfoRenderer(),
                structlog.processors.format_exc_info,
                structlog.processors.UnicodeDecoder(),
                structlog.processors.JSONRenderer()
            ],
            context_class=dict,
            logger_factory=structlog.stdlib.LoggerFactory(),
            wrapper_class=structlog.stdlib.BoundLogger,
            cache_logger_on_first_use=True,
        )
    else:
        structlog.configure(
            processors=[
                structlog.stdlib.filter_by_level,
                structlog.stdlib.add_logger_name,
                structlog.stdlib.add_log_level,
                structlog.stdlib.PositionalArgumentsFormatter(),
                structlog.processors.TimeStamper(fmt="iso"),
                structlog.processors.StackInfoRenderer(),
                structlog.processors.format_exc_info,
                structlog.processors.UnicodeDecoder(),
                structlog.dev.ConsoleRenderer()
            ],
            context_class=dict,
            logger_factory=structlog.stdlib.LoggerFactory(),
            wrapper_class=structlog.stdlib.BoundLogger,
            cache_logger_on_first_use=True,
        )

    logging.basicConfig(
        format="%(message)s",
        stream=sys.stdout,
        level=log_level,
    )


def parse_args():
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(description="HCS Vision Service")
    parser.add_argument(
        "--config", "-c",
        type=str,
        help="Path to configuration TOML file"
    )
    parser.add_argument(
        "--host",
        type=str,
        default="0.0.0.0",
        help="Host to bind to"
    )
    parser.add_argument(
        "--port", "-p",
        type=int,
        default=50051,
        help="Port to bind to"
    )
    parser.add_argument(
        "--workers", "-w",
        type=int,
        default=4,
        help="Number of worker threads"
    )
    parser.add_argument(
        "--log-level",
        type=str,
        choices=["debug", "info", "warning", "error"],
        default="info",
        help="Log level"
    )
    return parser.parse_args()


def main():
    args = parse_args()

    # Load config
    config = load_config(args.config)

    # Override with CLI args
    if args.host:
        config.service.host = args.host
    if args.port:
        config.service.port = args.port
    if args.workers:
        config.service.workers = args.workers
    if args.log_level:
        config.service.log_level = args.log_level

    # Setup logging
    setup_logging(config)
    logger = structlog.get_logger()

    logger.info("Starting vision service", host=config.service.host, port=config.service.port)

    # Start server
    server = serve(config, config.service.host, config.service.port, config.service.workers)

    # Handle shutdown
    def shutdown(signum, frame):
        logger.info("Shutting down...")
        server.stop(grace=5)
        sys.exit(0)

    signal.signal(signal.SIGINT, shutdown)
    signal.signal(signal.SIGTERM, shutdown)

    try:
        while True:
            time.sleep(86400)  # Sleep for a day
    except KeyboardInterrupt:
        shutdown(None, None)


if __name__ == "__main__":
    main()