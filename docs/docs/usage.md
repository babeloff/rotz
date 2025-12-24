---
sidebar_position: 2
title: Usage
---

Run `rotz --help` to see all commands Rotz has.

## Enhanced Copy Features

Rotz now supports enhanced copy functionality with intelligent file synchronization and optional encryption support for sensitive dotfiles. This feature provides more control over how files are managed compared to traditional symbolic or hard links.

### Key Features

- **Link Type System**: Five link types with different behaviors (symbolic, hard, copy, record, encrypted)
- **Intelligent Synchronization**: Automatic bidirectional sync based on file existence and link type
- **Encryption Support**: Optional age encryption for sensitive files (requires `encryption` feature)
- **Backward Compatibility**: All existing configurations continue to work unchanged

### Quick Example

```yaml
# dot.yaml
links:
  # Traditional configuration (still works)
  .bashrc: ~/.bashrc
  
  # Enhanced configuration with type overrides
  .gitconfig:
    targets: ~/.gitconfig
    type: record  # Sync changes back from target
    
  .secrets:
    targets: ~/.secrets
    type: encrypted  # Automatic encryption/decryption
```

For detailed information about configuration and encryption setup, see the [Encryption documentation](/docs/encryption).