# Contributing to SomaSync

Thank you for your interest in contributing to SomaSync! We welcome contributions from the community and are excited to see what you'll build.

## Getting Started

### Prerequisites

- Rust 1.70+ 
- Git
- A GitHub account

### Development Setup

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/somasync.git
   cd somasync
   ```
3. Add the upstream remote:
   ```bash
   git remote add upstream https://github.com/dendritic-labs/somasync.git
   ```
4. Create a new branch for your changes:
   ```bash
   git checkout -b feature/your-feature-name
   ```

## Development Workflow

### Before You Start

1. Check existing issues and PRs to avoid duplicating work
2. For large changes, please open an issue first to discuss the approach
3. Ensure your Rust toolchain is up to date: `rustup update`

### Making Changes

1. **Write Tests**: All new features and bug fixes should include tests
2. **Follow Rust Conventions**: Use `cargo fmt` and `cargo clippy`
3. **Document Your Code**: Add documentation for public APIs
4. **Keep Commits Focused**: Each commit should represent a single logical change

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with all features
cargo test --all-features

# Run clippy for linting
cargo clippy --all-targets --all-features -- -D warnings

# Format code
cargo fmt

# Check documentation
cargo doc --no-deps --document-private-items
```

### Performance Testing

```bash
# Run benchmarks (if available)
cargo bench

# Profile memory usage
cargo test --release -- --nocapture
```

## Pull Request Process

### Before Submitting

- [ ] Tests pass locally (`cargo test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Documentation builds (`cargo doc`)
- [ ] CHANGELOG.md updated (if applicable)
- [ ] Commit messages are clear and descriptive

### PR Requirements

1. **Clear Description**: Explain what your PR does and why
2. **Link Issues**: Reference any related issues with "Fixes #123"
3. **Tests Included**: New features need tests
4. **Documentation**: Update docs for API changes
5. **Small Scope**: Keep PRs focused and reviewable

### PR Template

When creating a PR, please include:

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that causes existing functionality to change)
- [ ] Documentation update

## Testing
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Manual testing completed

## Checklist
- [ ] Code follows project style guidelines
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
```

## Issue Guidelines

### Bug Reports

Use the bug report template and include:
- Rust version (`rustc --version`)
- SomaSync version
- Operating system
- Minimal reproduction case
- Expected vs actual behavior

### Feature Requests

- Describe the problem you're trying to solve
- Explain why this feature would be valuable
- Consider backwards compatibility
- Suggest an implementation approach if possible

## Code Style

### Rust Conventions

- Follow standard Rust naming conventions
- Use `cargo fmt` with default settings
- Prefer explicit over implicit when it improves clarity
- Write self-documenting code with good variable names

### Documentation

- All public APIs must have documentation
- Include examples in doc comments when helpful
- Use proper markdown formatting
- Document error conditions and panics

### Testing

- Unit tests for individual functions/methods
- Integration tests for public APIs
- Property-based tests for complex algorithms
- Benchmark tests for performance-critical code

## 🚦 Commit Guidelines

### Commit Messages

Use conventional commits format:

```
type(scope): description

[optional body]

[optional footer]
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

**Examples:**
```
feat(gossip): add message deduplication
fix(peer): handle connection timeout properly
docs(api): update SynapseNode examples
```

### Branch Naming

- `feature/short-description` - New features
- `fix/short-description` - Bug fixes
- `docs/short-description` - Documentation updates
- `refactor/short-description` - Code refactoring

## Recognition

Contributors are recognized in several ways:

- Listed in CONTRIBUTORS.md
- Mentioned in release notes for significant contributions
- GitHub's contributor statistics
- Special recognition for long-term contributors

## 📞 Getting Help

- **Questions**: Open a discussion on GitHub
- **Issues**: Create an issue with appropriate labels
- **Chat**: Join our community discussions
- **Email**: Contact maintainers for sensitive issues

## 📜 Code of Conduct

This project follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). 
By participating, you agree to uphold this code.

## Security

For security-related issues, please see [SECURITY.md](SECURITY.md) for responsible disclosure guidelines.

---

Thank you for contributing to SomaSync!