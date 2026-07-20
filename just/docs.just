import 'common.just'

mod tooling

run := "MISE_ENV=node ./scripts/docker/run-tooling.sh " + node_tooling_image
inputs := "README.md AGENTS.md CLAUDE.md 'docs/**/*.md'"

# Check project-owned Markdown against the repository style configuration
lint: tooling::build-node
    {{ run }} markdownlint-cli2 {{ inputs }}

# Check Markdown style and local links without making network requests
check: lint tooling::build-node
    {{ run }} lychee --offline {{ inputs }}

# Check every local and external link; use GITHUB_TOKEN when available to avoid rate limits
links: tooling::build-node
    {{ run }} lychee {{ inputs }}
