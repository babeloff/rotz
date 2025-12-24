# Rotz Examples

This directory contains example configurations demonstrating various features of Rotz.

## Files

### enhanced-copy.yaml

Demonstrates the new enhanced copy functionality with:

- **Link Type System**: Five link types with different behaviors
  - `symbolic`: Creates symbolic links (default)
  - `hard`: Creates hard links
  - `copy`: Copies files from dotfiles to target
  - `record`: Copies files with reverse sync (records changes from external apps)
  - `encrypted`: Copies files with encryption support

- **Type Override System**: Individual files can override the global `link_type`

- **Intelligent Synchronization**: Automatic bidirectional sync based on file existence and link type

- **Encryption Support**: Age-based encryption for sensitive dotfiles

### Usage

1. Copy the example configuration to your dotfiles directory:
```bash
cp enhanced-copy.yaml ~/.dotfiles/dot.yaml
```

2. For encryption features, generate age keys:
```bash
# Generate a new age key pair
age-keygen > ~/.age/key.txt

# Extract the public key and update the config
age-keygen -y ~/.age/key.txt
```

3. Set your global link type in config.yaml:
```bash
echo "link_type: copy" >> ~/.config/rotz/config.yaml
```

4. Update the configuration with your actual file paths and age public key

5. Link your dotfiles:
```bash
rotz link
```

## Type Override System

Individual files can override the global `link_type` setting:

```yaml
# config.yaml
link_type: copy  # Global default

# dot.yaml  
links:
  .bashrc: ~/.bashrc           # Uses global (copy)
  .gitconfig:                  # Override to record
    targets: ~/.gitconfig
    type: record
  .secrets:                    # Override to encrypted
    targets: ~/.secrets  
    type: encrypted
```

## Synchronization Behavior

The link type determines both linking method and sync behavior:

| Source Exists | Target Exists | Link Type | Sync Direction |
|---------------|---------------|-----------|----------------|
| No            | No            | Any       | Create both empty |
| No            | Yes           | Any       | Target → Source |
| Yes           | No            | Any       | Source → Target |
| Yes           | Yes           | symbolic/hard/copy | Source → Target |
| Yes           | Yes           | record    | Target → Source |
| Yes           | Yes           | encrypted | Target → Source (with encryption) |

## Security Notes

- Keep your age private key secure and backed up
- Encrypted files in your dotfiles repository are only as secure as your key management
- Consider using different keys for different security contexts
- The `type` field overrides the global `link_type` for individual files