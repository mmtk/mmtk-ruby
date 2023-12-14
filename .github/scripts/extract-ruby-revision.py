import argparse
import tomlkit
import os

parser = argparse.ArgumentParser(
        description='Extract the Ruby repo revision to test against',
        )

parser.add_argument('toml_path', help='Path to Cargo.toml')
parser.add_argument('--github-output', help='Print in the format of GITHUB_OUTPUT')

args = parser.parse_args()

with open(args.toml_path, "rb") as f:
    toml_data = tomlkit.load(f)

ruby_node = toml_data["package"]["metadata"]["ci-repos"]["ruby"]
repo = ruby_node["repo"]
rev = ruby_node["rev"]
print(f"ruby_repo={repo}")
print(f"ruby_rev={rev}")

if args.github_output is not None:
    print(f"Writing to GITHUB_OUTPUT: {args.github_output}")
    with open(args.github_output, "at") as f:
        print(f"ruby_repo={repo}", file=f)
        print(f"ruby_rev={rev}", file=f)
