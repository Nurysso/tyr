# Copyright (C) 2026 Dawood Khan
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Maintainer Dawood (Nurysso) contact - nurysso [at] proton.me
#
# This file is responsible for creatinf dummy file for
#  tyr(https:github.com/Nurysso/tyr) project to test its functionality

import os
import random
import string
import argparse
import json
from concurrent.futures import ThreadPoolExecutor

# Sample mock data pools for generating "meaningful" content
LOG_LEVELS = ["INFO", "WARNING", "ERROR", "DEBUG"]
USER_AGENTS = ["Mozilla/5.0", "Chrome/120.0", "Safari/13.1", "Edge/92.0"]
PRODUCTS = ["Laptop", "Smartphone", "Headphones", "Smartwatch", "Tablet"]

def rand_name(length=8):
    return "".join(random.choices(string.ascii_lowercase + string.digits, k=length))

def generate_meaningful_content(ext, target_size):
    """Generates semi-realistic text or structured data up to the target size."""
    lines = []
    current_size = 0

    # Pre-bake some reusable text blocks to save CPU cycles
    words = ["lorem", "ipsum", "dolor", "sit", "amet", "consectetur", "adipiscing", "elit", "data", "system"]

    while current_size < target_size:
        if ext in [".txt", ".md"]:
            line = " ".join(random.choices(words, k=15)) + ".\n"
        elif ext == ".log":
            line = f"2026-05-24 {random.choice(LOG_LEVELS)}: User_{random.randint(100,999)} accessed route /{rand_name(4)} from {random.choice(USER_AGENTS)}\n"
        elif ext == ".csv":
            line = f"{random.randint(1000,9999)},{random.choice(PRODUCTS)},{random.choice(LOG_LEVELS)},{random.randint(1,5)},{random.uniform(10.0, 500.0):.2f}\n"
        elif ext in [".json", ".xml"]:
            # Structured data formatted as lines for easy size checking
            obj = {"id": random.randint(1000, 9999), "type": random.choice(PRODUCTS), "status": random.choice(LOG_LEVELS), "hash": rand_name(12)}
            line = json.dumps(obj) + ",\n" if ext == ".json" else f"<record id='{obj['id']}'><type>{obj['type']}</type></record>\n"
        else: # .dat, .bin fall back to fast pseudo-random text
            line = rand_name(50) + "\n"

        encoded_line = line.encode('utf-8')
        lines.append(encoded_line)
        current_size += len(encoded_line)

    return b"".join(lines)[:target_size] # Trim exactly to target size

def create_single_file(file_id, output_dir, extensions, min_bytes, max_bytes):
    """Worker function executed by threads."""
    ext = random.choice(extensions)
    size = random.randint(min_bytes, max_bytes)
    path = os.path.join(output_dir, f"{rand_name()}{ext}")

    # Generate content based on extension type
    content = generate_meaningful_content(ext, size)

    with open(path, "wb") as f:
        f.write(content)

    print(f"[{file_id:>4}] Created: {path} ({size:,} bytes)")

def main():
    # Setup Command Line Arguments
    parser = argparse.ArgumentParser(description="Fast Multi-threaded Dummy File Generator.")
    parser.add_argument("-n", "--num-files", type=int, default=100, help="Number of files to generate (Default: 100)")
    parser.add_argument("-o", "--output", type=str, default="dummy_files", help="Output directory name")
    parser.add_argument("-t", "--threads", type=int, default=4, help="Number of concurrent threads (Default: 4)")
    parser.add_argument("--min-mb", type=float, default=0.1, help="Minimum file size in MB")
    parser.add_argument("--max-mb", type=float, default=5.0, help="Maximum file size in MB")

    args = parser.parse_args()

    # Convert sizes from MB to Bytes
    min_bytes = int(args.min_mb * 1024 * 1024)
    max_bytes = int(args.max_mb * 1024 * 1024)
    extensions = [".txt", ".log", ".csv", ".json", ".xml", ".md", ".dat", ".bin"]

    os.makedirs(args.output, exist_ok=True)

    print(f"Generating {args.num_files} files using {args.threads} threads...\n")

    # Multithreading Execution
    with ThreadPoolExecutor(max_workers=args.threads) as executor:
        # Submit tasks to the thread pool
        futures = [
            executor.submit(create_single_file, i + 1, args.output, extensions, min_bytes, max_bytes)
            for i in range(args.num_files)
        ]

    print(f"\nDone! All files written to '{args.output}/'")

if __name__ == "__main__":
    main()
