//! System Administration Task Pattern Detection
//!
//! This module provides pattern detection and guidance for system administration
//! tasks including VM management, disk operations, network configuration, and
//! OS installation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Categories of system administration tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SysadminCategory {
    /// Virtual machine management (QEMU, VirtualBox, VMware)
    Virtualization,
    /// Disk image creation, manipulation, and management
    DiskManagement,
    /// Network configuration and troubleshooting
    Networking,
    /// Operating system installation and setup
    OsInstallation,
    /// Boot process and bootloader configuration
    BootConfiguration,
    /// Package management and software installation
    PackageManagement,
    /// Service and daemon management
    ServiceManagement,
    /// User, group, and permissions management
    UserPermissions,
    /// System monitoring and performance tuning
    SystemMonitoring,
    /// Backup and recovery operations
    BackupRecovery,
}

/// Detailed guidance for implementing a sysadmin task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SysadminGuidance {
    /// High-level approach description
    pub approach: String,
    /// Required tools and utilities
    pub required_tools: Vec<String>,
    /// Step-by-step implementation guide
    pub steps: Vec<String>,
    /// Common pitfalls to avoid
    pub pitfalls: Vec<String>,
    /// Safety considerations
    pub safety_notes: Vec<String>,
    /// Example command snippets
    pub example_commands: Option<Vec<String>>,
}

/// Specific system administration pattern with implementation guidance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SysadminPattern {
    /// Category of the pattern
    pub category: SysadminCategory,
    /// Name of the pattern
    pub name: String,
    /// Keywords that identify this pattern
    pub keywords: Vec<String>,
    /// Characteristics that indicate this pattern applies
    pub characteristics: Vec<String>,
    /// Confidence score for detection (0.0 to 1.0)
    pub confidence: f64,
    /// Detailed guidance for this pattern
    pub guidance: SysadminGuidance,
}

/// Result of pattern detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SysadminDetectionResult {
    /// Detected patterns, sorted by confidence
    pub patterns: Vec<SysadminPattern>,
    /// Keywords found in the task description
    pub matched_keywords: Vec<String>,
    /// Overall confidence that this is a sysadmin task
    pub overall_confidence: f64,
    /// Whether prompt augmentation is recommended
    pub should_augment: bool,
}

/// System administration pattern detector
pub struct SysadminPatternDetector {
    patterns: Vec<SysadminPattern>,
}

impl Default for SysadminPatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl SysadminPatternDetector {
    /// Create a new pattern detector with built-in patterns
    pub fn new() -> Self {
        Self {
            patterns: Self::build_default_patterns(),
        }
    }

    /// Detect patterns in a task description
    pub fn detect(&self, task_description: &str) -> SysadminDetectionResult {
        let lower_desc = task_description.to_lowercase();
        let mut matched_patterns: Vec<SysadminPattern> = Vec::new();
        let mut matched_keywords: Vec<String> = Vec::new();

        for pattern in &self.patterns {
            let mut keyword_matches = 0;
            let mut pattern_keywords = Vec::new();

            for keyword in &pattern.keywords {
                if lower_desc.contains(&keyword.to_lowercase()) {
                    keyword_matches += 1;
                    pattern_keywords.push(keyword.clone());
                }
            }

            let mut characteristic_matches = 0;
            for characteristic in &pattern.characteristics {
                if lower_desc.contains(&characteristic.to_lowercase()) {
                    characteristic_matches += 1;
                }
            }

            if keyword_matches > 0 || characteristic_matches > 0 {
                let keyword_score = keyword_matches as f64 / pattern.keywords.len().max(1) as f64;
                let char_score =
                    characteristic_matches as f64 / pattern.characteristics.len().max(1) as f64;
                let confidence = (keyword_score * 0.7 + char_score * 0.3).min(1.0);

                if confidence > 0.1 {
                    let mut matched_pattern = pattern.clone();
                    matched_pattern.confidence = confidence;
                    matched_patterns.push(matched_pattern);
                    matched_keywords.extend(pattern_keywords);
                }
            }
        }

        // Sort by confidence (highest first)
        matched_patterns.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Deduplicate keywords
        matched_keywords.sort();
        matched_keywords.dedup();

        let overall_confidence = matched_patterns
            .first()
            .map(|p| p.confidence)
            .unwrap_or(0.0);

        SysadminDetectionResult {
            patterns: matched_patterns,
            matched_keywords,
            overall_confidence,
            should_augment: overall_confidence > 0.2,
        }
    }

    /// Generate prompt augmentation text based on detection results
    pub fn generate_prompt_augmentation(&self, detection: &SysadminDetectionResult) -> String {
        if !detection.should_augment || detection.patterns.is_empty() {
            return String::new();
        }

        let mut augmentation = String::new();
        augmentation.push_str("\n\n## System Administration Task Guidance\n\n");

        for (idx, pattern) in detection.patterns.iter().take(2).enumerate() {
            if idx > 0 {
                augmentation.push_str("\n---\n\n");
            }

            augmentation.push_str(&format!(
                "### Detected Pattern: {} ({:?})\n\n",
                pattern.name, pattern.category
            ));

            augmentation.push_str(&format!("**Approach**: {}\n\n", pattern.guidance.approach));

            if !pattern.guidance.required_tools.is_empty() {
                augmentation.push_str("**Required Tools**:\n");
                for tool in &pattern.guidance.required_tools {
                    augmentation.push_str(&format!("- `{}`\n", tool));
                }
                augmentation.push('\n');
            }

            augmentation.push_str("**Implementation Steps**:\n");
            for (i, step) in pattern.guidance.steps.iter().enumerate() {
                augmentation.push_str(&format!("{}. {}\n", i + 1, step));
            }
            augmentation.push('\n');

            if !pattern.guidance.pitfalls.is_empty() {
                augmentation.push_str("**Common Pitfalls**:\n");
                for pitfall in &pattern.guidance.pitfalls {
                    augmentation.push_str(&format!("- ⚠️ {}\n", pitfall));
                }
                augmentation.push('\n');
            }

            if !pattern.guidance.safety_notes.is_empty() {
                augmentation.push_str("**Safety Notes**:\n");
                for note in &pattern.guidance.safety_notes {
                    augmentation.push_str(&format!("- 🔒 {}\n", note));
                }
                augmentation.push('\n');
            }

            if let Some(commands) = &pattern.guidance.example_commands {
                augmentation.push_str("**Example Commands**:\n```bash\n");
                for cmd in commands {
                    augmentation.push_str(&format!("{}\n", cmd));
                }
                augmentation.push_str("```\n");
            }
        }

        augmentation
    }

    /// Build the default set of sysadmin patterns
    fn build_default_patterns() -> Vec<SysadminPattern> {
        vec![
            // QEMU/VM Management
            SysadminPattern {
                category: SysadminCategory::Virtualization,
                name: "QEMU Virtual Machine Management".to_string(),
                keywords: vec![
                    "qemu".to_string(),
                    "kvm".to_string(),
                    "virtual machine".to_string(),
                    "vm".to_string(),
                    "virtualization".to_string(),
                    "qemu-system".to_string(),
                    "qemu-img".to_string(),
                    "hypervisor".to_string(),
                ],
                characteristics: vec![
                    "create vm".to_string(),
                    "run vm".to_string(),
                    "start vm".to_string(),
                    "install os".to_string(),
                    "boot from iso".to_string(),
                    "emulate".to_string(),
                ],
                confidence: 0.0,
                guidance: SysadminGuidance {
                    approach: "Use QEMU to create and manage virtual machines. For better performance, enable KVM if available on Linux hosts.".to_string(),
                    required_tools: vec![
                        "qemu-system-x86_64".to_string(),
                        "qemu-img".to_string(),
                        "qemu-nbd (optional)".to_string(),
                    ],
                    steps: vec![
                        "Create a disk image: qemu-img create -f qcow2 disk.qcow2 20G".to_string(),
                        "Boot from ISO for installation: qemu-system-x86_64 -cdrom install.iso -hda disk.qcow2 -boot d -m 2G".to_string(),
                        "After installation, boot from disk: qemu-system-x86_64 -hda disk.qcow2 -m 2G".to_string(),
                        "For KVM acceleration (Linux): add -enable-kvm flag".to_string(),
                        "Configure networking as needed (user mode, bridge, etc.)".to_string(),
                    ],
                    pitfalls: vec![
                        "Forgetting to allocate enough RAM (-m flag)".to_string(),
                        "Not enabling KVM when available (significantly slower without it)".to_string(),
                        "Using raw disk format instead of qcow2 (loses snapshot capability)".to_string(),
                        "Boot order issues - use -boot flag to specify boot device".to_string(),
                    ],
                    safety_notes: vec![
                        "VMs are isolated but can still access host network".to_string(),
                        "Disk images can grow large - monitor disk space".to_string(),
                        "Snapshots consume disk space - clean up old snapshots".to_string(),
                    ],
                    example_commands: Some(vec![
                        "# Create a 20GB qcow2 disk image".to_string(),
                        "qemu-img create -f qcow2 disk.qcow2 20G".to_string(),
                        "".to_string(),
                        "# Boot from ISO with 2GB RAM".to_string(),
                        "qemu-system-x86_64 -cdrom install.iso -hda disk.qcow2 -boot d -m 2G -enable-kvm".to_string(),
                        "".to_string(),
                        "# Normal boot after installation".to_string(),
                        "qemu-system-x86_64 -hda disk.qcow2 -m 2G -enable-kvm".to_string(),
                    ]),
                },
            },
            // VirtualBox Management
            SysadminPattern {
                category: SysadminCategory::Virtualization,
                name: "VirtualBox VM Management".to_string(),
                keywords: vec![
                    "virtualbox".to_string(),
                    "vbox".to_string(),
                    "vboxmanage".to_string(),
                    "oracle vm".to_string(),
                ],
                characteristics: vec![
                    "headless".to_string(),
                    "guest additions".to_string(),
                    "shared folder".to_string(),
                ],
                confidence: 0.0,
                guidance: SysadminGuidance {
                    approach: "Use VBoxManage CLI for automation or VirtualBox GUI for interactive use.".to_string(),
                    required_tools: vec![
                        "VBoxManage".to_string(),
                        "VirtualBox".to_string(),
                    ],
                    steps: vec![
                        "Create VM: VBoxManage createvm --name 'MyVM' --ostype Linux_64 --register".to_string(),
                        "Configure memory: VBoxManage modifyvm 'MyVM' --memory 2048".to_string(),
                        "Create and attach storage".to_string(),
                        "Mount ISO and start installation".to_string(),
                        "Install guest additions for better integration".to_string(),
                    ],
                    pitfalls: vec![
                        "Not installing guest additions (poor graphics, no shared folders)".to_string(),
                        "Network adapter type mismatch".to_string(),
                        "Forgetting to enable hardware virtualization in BIOS".to_string(),
                    ],
                    safety_notes: vec![
                        "Keep VirtualBox and guest additions updated".to_string(),
                        "Be careful with shared folder permissions".to_string(),
                    ],
                    example_commands: Some(vec![
                        "VBoxManage createvm --name 'MyVM' --ostype Linux_64 --register".to_string(),
                        "VBoxManage modifyvm 'MyVM' --memory 2048 --cpus 2".to_string(),
                        "VBoxManage startvm 'MyVM' --type headless".to_string(),
                    ]),
                },
            },
            // Disk Image Management
            SysadminPattern {
                category: SysadminCategory::DiskManagement,
                name: "Disk Image Operations".to_string(),
                keywords: vec![
                    "disk image".to_string(),
                    "qcow2".to_string(),
                    "raw image".to_string(),
                    "vdi".to_string(),
                    "vmdk".to_string(),
                    "iso".to_string(),
                    "dd".to_string(),
                    "img".to_string(),
                ],
                characteristics: vec![
                    "create image".to_string(),
                    "convert image".to_string(),
                    "resize disk".to_string(),
                    "mount image".to_string(),
                    "clone disk".to_string(),
                ],
                confidence: 0.0,
                guidance: SysadminGuidance {
                    approach: "Use appropriate tools based on image format. qemu-img for VM images, dd for raw operations, losetup for mounting.".to_string(),
                    required_tools: vec![
                        "qemu-img".to_string(),
                        "dd".to_string(),
                        "losetup".to_string(),
                        "mount".to_string(),
                        "fdisk/parted".to_string(),
                    ],
                    steps: vec![
                        "Identify the disk image format and target format".to_string(),
                        "For conversion: qemu-img convert -f <source> -O <dest> input output".to_string(),
                        "For mounting: use losetup to create loop device, then mount".to_string(),
                        "For resizing: qemu-img resize image.qcow2 +10G".to_string(),
                        "After resizing, expand the filesystem inside the image".to_string(),
                    ],
                    pitfalls: vec![
                        "Not backing up before operations".to_string(),
                        "Forgetting to expand filesystem after resizing image".to_string(),
                        "Using wrong block size with dd (slow or corrupted)".to_string(),
                        "Not unmounting before operations".to_string(),
                    ],
                    safety_notes: vec![
                        "Always backup important images before modification".to_string(),
                        "Double-check device names with dd to avoid data loss".to_string(),
                        "Use sync after dd operations".to_string(),
                    ],
                    example_commands: Some(vec![
                        "# Convert raw to qcow2".to_string(),
                        "qemu-img convert -f raw -O qcow2 disk.raw disk.qcow2".to_string(),
                        "".to_string(),
                        "# Resize qcow2 image".to_string(),
                        "qemu-img resize disk.qcow2 +10G".to_string(),
                        "".to_string(),
                        "# Mount a raw disk image".to_string(),
                        "sudo losetup -fP disk.img".to_string(),
                        "sudo mount /dev/loop0p1 /mnt".to_string(),
                    ]),
                },
            },
            // Network Configuration
            SysadminPattern {
                category: SysadminCategory::Networking,
                name: "Network Configuration".to_string(),
                keywords: vec![
                    "network".to_string(),
                    "ip address".to_string(),
                    "interface".to_string(),
                    "dhcp".to_string(),
                    "static ip".to_string(),
                    "bridge".to_string(),
                    "vlan".to_string(),
                    "firewall".to_string(),
                    "iptables".to_string(),
                    "netplan".to_string(),
                ],
                characteristics: vec![
                    "configure network".to_string(),
                    "set ip".to_string(),
                    "network interface".to_string(),
                    "routing".to_string(),
                    "dns".to_string(),
                ],
                confidence: 0.0,
                guidance: SysadminGuidance {
                    approach: "Use appropriate network management tool for your distribution (netplan for Ubuntu, NetworkManager, or direct ip commands).".to_string(),
                    required_tools: vec![
                        "ip".to_string(),
                        "netplan (Ubuntu)".to_string(),
                        "nmcli (NetworkManager)".to_string(),
                        "iptables/nftables".to_string(),
                    ],
                    steps: vec![
                        "Identify current network configuration: ip addr, ip route".to_string(),
                        "Determine the configuration method (netplan, NetworkManager, etc.)".to_string(),
                        "Edit configuration files or use CLI tools".to_string(),
                        "Apply changes and verify connectivity".to_string(),
                        "Configure DNS resolution if needed".to_string(),
                    ],
                    pitfalls: vec![
                        "Locking yourself out when configuring remote systems".to_string(),
                        "Conflicting network managers".to_string(),
                        "Forgetting to persist changes".to_string(),
                        "DNS resolution issues after changes".to_string(),
                    ],
                    safety_notes: vec![
                        "Test changes with a timeout or have console access".to_string(),
                        "Document original configuration before changes".to_string(),
                        "Use screen/tmux for remote configuration changes".to_string(),
                    ],
                    example_commands: Some(vec![
                        "# View current configuration".to_string(),
                        "ip addr show".to_string(),
                        "ip route show".to_string(),
                        "".to_string(),
                        "# Temporary IP assignment".to_string(),
                        "sudo ip addr add 192.168.1.100/24 dev eth0".to_string(),
                        "".to_string(),
                        "# Apply netplan configuration".to_string(),
                        "sudo netplan apply".to_string(),
                    ]),
                },
            },
            // OS Installation
            SysadminPattern {
                category: SysadminCategory::OsInstallation,
                name: "Operating System Installation".to_string(),
                keywords: vec![
                    "install".to_string(),
                    "installation".to_string(),
                    "operating system".to_string(),
                    "os".to_string(),
                    "linux".to_string(),
                    "windows".to_string(),
                    "ubuntu".to_string(),
                    "debian".to_string(),
                    "centos".to_string(),
                    "fedora".to_string(),
                ],
                characteristics: vec![
                    "install os".to_string(),
                    "boot from".to_string(),
                    "bootable usb".to_string(),
                    "setup wizard".to_string(),
                    "partitioning".to_string(),
                ],
                confidence: 0.0,
                guidance: SysadminGuidance {
                    approach: "Download official ISO, create bootable media, boot and follow installation wizard. For VMs, attach ISO directly.".to_string(),
                    required_tools: vec![
                        "dd or rufus/balenaEtcher (for USB)".to_string(),
                        "QEMU/VirtualBox (for VMs)".to_string(),
                        "ISO image".to_string(),
                    ],
                    steps: vec![
                        "Download official ISO from distribution website".to_string(),
                        "Verify checksum of downloaded ISO".to_string(),
                        "Create bootable USB or attach ISO to VM".to_string(),
                        "Boot from installation media".to_string(),
                        "Follow installation wizard, configure partitions".to_string(),
                        "Set up user account and timezone".to_string(),
                        "Complete installation and reboot".to_string(),
                        "Install updates and additional software".to_string(),
                    ],
                    pitfalls: vec![
                        "Not verifying ISO checksum (could be corrupted or malicious)".to_string(),
                        "Incorrect partition scheme (MBR vs GPT)".to_string(),
                        "Not setting up bootloader correctly".to_string(),
                        "Overwriting existing data unintentionally".to_string(),
                    ],
                    safety_notes: vec![
                        "Backup all data before installation on physical hardware".to_string(),
                        "Verify you're installing to the correct disk".to_string(),
                        "Keep installation media available for recovery".to_string(),
                    ],
                    example_commands: Some(vec![
                        "# Verify ISO checksum".to_string(),
                        "sha256sum ubuntu-22.04.iso".to_string(),
                        "".to_string(),
                        "# Create bootable USB (Linux)".to_string(),
                        "sudo dd if=ubuntu-22.04.iso of=/dev/sdX bs=4M status=progress".to_string(),
                        "sync".to_string(),
                    ]),
                },
            },
            // Boot Configuration
            SysadminPattern {
                category: SysadminCategory::BootConfiguration,
                name: "Bootloader Configuration".to_string(),
                keywords: vec![
                    "grub".to_string(),
                    "bootloader".to_string(),
                    "boot".to_string(),
                    "efi".to_string(),
                    "uefi".to_string(),
                    "mbr".to_string(),
                    "bios".to_string(),
                    "systemd-boot".to_string(),
                ],
                characteristics: vec![
                    "boot menu".to_string(),
                    "dual boot".to_string(),
                    "boot repair".to_string(),
                    "kernel parameters".to_string(),
                ],
                confidence: 0.0,
                guidance: SysadminGuidance {
                    approach: "Configure bootloader (usually GRUB) through its configuration files. Always keep a backup boot method available.".to_string(),
                    required_tools: vec![
                        "grub-install".to_string(),
                        "update-grub".to_string(),
                        "efibootmgr (for UEFI)".to_string(),
                    ],
                    steps: vec![
                        "Identify boot mode (BIOS/Legacy vs UEFI)".to_string(),
                        "Edit /etc/default/grub for configuration changes".to_string(),
                        "Run update-grub to regenerate config".to_string(),
                        "For UEFI: use efibootmgr to manage boot entries".to_string(),
                        "Test boot configuration before making permanent".to_string(),
                    ],
                    pitfalls: vec![
                        "Not running update-grub after changes".to_string(),
                        "Mixing BIOS and UEFI installations".to_string(),
                        "Incorrect root= parameter making system unbootable".to_string(),
                        "Deleting EFI partition accidentally".to_string(),
                    ],
                    safety_notes: vec![
                        "Keep a live USB for boot repair".to_string(),
                        "Document working boot configuration before changes".to_string(),
                        "Test in VM before applying to production".to_string(),
                    ],
                    example_commands: Some(vec![
                        "# Edit GRUB configuration".to_string(),
                        "sudo nano /etc/default/grub".to_string(),
                        "sudo update-grub".to_string(),
                        "".to_string(),
                        "# Reinstall GRUB to MBR".to_string(),
                        "sudo grub-install /dev/sda".to_string(),
                        "".to_string(),
                        "# List UEFI boot entries".to_string(),
                        "efibootmgr -v".to_string(),
                    ]),
                },
            },
            // Package Management
            SysadminPattern {
                category: SysadminCategory::PackageManagement,
                name: "Package Management".to_string(),
                keywords: vec![
                    "apt".to_string(),
                    "yum".to_string(),
                    "dnf".to_string(),
                    "pacman".to_string(),
                    "package".to_string(),
                    "install software".to_string(),
                    "repository".to_string(),
                    "dependency".to_string(),
                ],
                characteristics: vec![
                    "install package".to_string(),
                    "update system".to_string(),
                    "remove software".to_string(),
                    "add repository".to_string(),
                ],
                confidence: 0.0,
                guidance: SysadminGuidance {
                    approach: "Use the distribution's package manager. Always update package lists before installing.".to_string(),
                    required_tools: vec![
                        "apt/apt-get (Debian/Ubuntu)".to_string(),
                        "dnf/yum (RHEL/Fedora)".to_string(),
                        "pacman (Arch)".to_string(),
                        "zypper (openSUSE)".to_string(),
                    ],
                    steps: vec![
                        "Update package lists: apt update / dnf check-update".to_string(),
                        "Search for package: apt search <name>".to_string(),
                        "Install package: apt install <name>".to_string(),
                        "Remove package: apt remove <name>".to_string(),
                        "Upgrade all packages: apt upgrade".to_string(),
                    ],
                    pitfalls: vec![
                        "Not updating package lists before install".to_string(),
                        "Removing packages that other packages depend on".to_string(),
                        "Adding untrusted repositories".to_string(),
                        "Interrupting package operations".to_string(),
                    ],
                    safety_notes: vec![
                        "Only add trusted repositories".to_string(),
                        "Review what will be installed/removed before confirming".to_string(),
                        "Keep system updated for security patches".to_string(),
                    ],
                    example_commands: Some(vec![
                        "# Debian/Ubuntu".to_string(),
                        "sudo apt update && sudo apt upgrade".to_string(),
                        "sudo apt install nginx".to_string(),
                        "".to_string(),
                        "# RHEL/Fedora".to_string(),
                        "sudo dnf update".to_string(),
                        "sudo dnf install nginx".to_string(),
                    ]),
                },
            },
            // Service Management
            SysadminPattern {
                category: SysadminCategory::ServiceManagement,
                name: "Service and Daemon Management".to_string(),
                keywords: vec![
                    "systemd".to_string(),
                    "service".to_string(),
                    "daemon".to_string(),
                    "systemctl".to_string(),
                    "init".to_string(),
                    "unit".to_string(),
                ],
                characteristics: vec![
                    "start service".to_string(),
                    "stop service".to_string(),
                    "enable service".to_string(),
                    "service status".to_string(),
                    "restart".to_string(),
                ],
                confidence: 0.0,
                guidance: SysadminGuidance {
                    approach: "Use systemctl on modern Linux systems. Check service status before and after changes.".to_string(),
                    required_tools: vec![
                        "systemctl".to_string(),
                        "journalctl".to_string(),
                    ],
                    steps: vec![
                        "Check service status: systemctl status <service>".to_string(),
                        "Start/stop service: systemctl start/stop <service>".to_string(),
                        "Enable at boot: systemctl enable <service>".to_string(),
                        "View logs: journalctl -u <service>".to_string(),
                        "Reload configuration: systemctl reload <service>".to_string(),
                    ],
                    pitfalls: vec![
                        "Forgetting to enable service for boot".to_string(),
                        "Not checking logs when service fails".to_string(),
                        "Using restart instead of reload when possible".to_string(),
                    ],
                    safety_notes: vec![
                        "Check dependencies before stopping services".to_string(),
                        "Test configuration before reloading".to_string(),
                        "Monitor service after changes".to_string(),
                    ],
                    example_commands: Some(vec![
                        "systemctl status nginx".to_string(),
                        "sudo systemctl start nginx".to_string(),
                        "sudo systemctl enable nginx".to_string(),
                        "journalctl -u nginx -f".to_string(),
                    ]),
                },
            },
            // User and Permissions
            SysadminPattern {
                category: SysadminCategory::UserPermissions,
                name: "User and Permission Management".to_string(),
                keywords: vec![
                    "user".to_string(),
                    "group".to_string(),
                    "permission".to_string(),
                    "chmod".to_string(),
                    "chown".to_string(),
                    "sudo".to_string(),
                    "useradd".to_string(),
                    "passwd".to_string(),
                ],
                characteristics: vec![
                    "create user".to_string(),
                    "add to group".to_string(),
                    "change permission".to_string(),
                    "set owner".to_string(),
                ],
                confidence: 0.0,
                guidance: SysadminGuidance {
                    approach: "Use appropriate commands for user management. Be careful with sudo and root access.".to_string(),
                    required_tools: vec![
                        "useradd/adduser".to_string(),
                        "usermod".to_string(),
                        "chmod".to_string(),
                        "chown".to_string(),
                        "visudo".to_string(),
                    ],
                    steps: vec![
                        "Create user: useradd -m -s /bin/bash username".to_string(),
                        "Set password: passwd username".to_string(),
                        "Add to group: usermod -aG groupname username".to_string(),
                        "Change ownership: chown user:group file".to_string(),
                        "Change permissions: chmod 755 file".to_string(),
                    ],
                    pitfalls: vec![
                        "Locking yourself out of sudo access".to_string(),
                        "Setting overly permissive permissions (777)".to_string(),
                        "Forgetting to set user shell".to_string(),
                        "Not creating home directory".to_string(),
                    ],
                    safety_notes: vec![
                        "Always use visudo for sudoers file".to_string(),
                        "Follow principle of least privilege".to_string(),
                        "Audit user accounts regularly".to_string(),
                    ],
                    example_commands: Some(vec![
                        "# Create user with home directory".to_string(),
                        "sudo useradd -m -s /bin/bash newuser".to_string(),
                        "sudo passwd newuser".to_string(),
                        "".to_string(),
                        "# Add user to sudo group".to_string(),
                        "sudo usermod -aG sudo newuser".to_string(),
                    ]),
                },
            },
            // System Monitoring
            SysadminPattern {
                category: SysadminCategory::SystemMonitoring,
                name: "System Monitoring and Performance".to_string(),
                keywords: vec![
                    "monitor".to_string(),
                    "performance".to_string(),
                    "cpu".to_string(),
                    "memory".to_string(),
                    "disk usage".to_string(),
                    "top".to_string(),
                    "htop".to_string(),
                    "free".to_string(),
                    "df".to_string(),
                ],
                characteristics: vec![
                    "check usage".to_string(),
                    "system load".to_string(),
                    "resource usage".to_string(),
                    "disk space".to_string(),
                ],
                confidence: 0.0,
                guidance: SysadminGuidance {
                    approach: "Use standard monitoring tools to check system health. Set up alerts for critical thresholds.".to_string(),
                    required_tools: vec![
                        "top/htop".to_string(),
                        "free".to_string(),
                        "df".to_string(),
                        "iostat".to_string(),
                        "vmstat".to_string(),
                    ],
                    steps: vec![
                        "Check CPU/memory: htop or top".to_string(),
                        "Check memory: free -h".to_string(),
                        "Check disk space: df -h".to_string(),
                        "Check disk I/O: iostat".to_string(),
                        "Check processes: ps aux".to_string(),
                    ],
                    pitfalls: vec![
                        "Ignoring warning signs until critical".to_string(),
                        "Not setting up monitoring alerts".to_string(),
                        "Misinterpreting load averages".to_string(),
                    ],
                    safety_notes: vec![
                        "Set up automated monitoring".to_string(),
                        "Keep historical data for trend analysis".to_string(),
                        "Have runbooks for common issues".to_string(),
                    ],
                    example_commands: Some(vec![
                        "htop".to_string(),
                        "free -h".to_string(),
                        "df -h".to_string(),
                        "iostat -x 1".to_string(),
                    ]),
                },
            },
            // Backup and Recovery
            SysadminPattern {
                category: SysadminCategory::BackupRecovery,
                name: "Backup and Recovery".to_string(),
                keywords: vec![
                    "backup".to_string(),
                    "restore".to_string(),
                    "recovery".to_string(),
                    "rsync".to_string(),
                    "tar".to_string(),
                    "snapshot".to_string(),
                ],
                characteristics: vec![
                    "create backup".to_string(),
                    "restore from".to_string(),
                    "archive".to_string(),
                    "replicate".to_string(),
                ],
                confidence: 0.0,
                guidance: SysadminGuidance {
                    approach: "Implement 3-2-1 backup rule: 3 copies, 2 different media, 1 offsite. Test restores regularly.".to_string(),
                    required_tools: vec![
                        "rsync".to_string(),
                        "tar".to_string(),
                        "borgbackup".to_string(),
                        "restic".to_string(),
                    ],
                    steps: vec![
                        "Identify critical data to backup".to_string(),
                        "Choose backup method (full, incremental, differential)".to_string(),
                        "Create backup: rsync -avz source/ dest/".to_string(),
                        "Verify backup integrity".to_string(),
                        "Test restore procedure".to_string(),
                        "Automate with cron or systemd timer".to_string(),
                    ],
                    pitfalls: vec![
                        "Not testing restore procedures".to_string(),
                        "Storing all backups in same location".to_string(),
                        "Not encrypting sensitive backups".to_string(),
                        "Forgetting to backup configuration files".to_string(),
                    ],
                    safety_notes: vec![
                        "Encrypt backups containing sensitive data".to_string(),
                        "Store backups in multiple locations".to_string(),
                        "Regularly test restore procedures".to_string(),
                        "Document backup and restore procedures".to_string(),
                    ],
                    example_commands: Some(vec![
                        "# Rsync backup".to_string(),
                        "rsync -avz --delete /data/ /backup/data/".to_string(),
                        "".to_string(),
                        "# Create compressed archive".to_string(),
                        "tar -czvf backup-$(date +%Y%m%d).tar.gz /data/".to_string(),
                    ]),
                },
            },
        ]
    }

    /// Add a custom pattern to the detector
    pub fn add_pattern(&mut self, pattern: SysadminPattern) {
        self.patterns.push(pattern);
    }

    /// Get all registered patterns
    pub fn get_patterns(&self) -> &[SysadminPattern] {
        &self.patterns
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_qemu_pattern() {
        let detector = SysadminPatternDetector::new();
        let result = detector.detect("Create a QEMU virtual machine to install Windows XP");

        assert!(
            !result.patterns.is_empty(),
            "Should detect at least one pattern"
        );
        let has_vm = result
            .patterns
            .iter()
            .any(|p| p.category == SysadminCategory::Virtualization || p.name.contains("QEMU"));
        assert!(has_vm, "Should detect VM/QEMU pattern");
    }

    #[test]
    fn test_detect_disk_image_pattern() {
        let detector = SysadminPatternDetector::new();
        let result = detector.detect("Convert raw disk image to qcow2 format");

        assert!(
            !result.patterns.is_empty(),
            "Should detect at least one pattern"
        );
        let has_disk = result
            .patterns
            .iter()
            .any(|p| p.category == SysadminCategory::DiskManagement);
        assert!(has_disk, "Should detect disk management pattern");
    }

    #[test]
    fn test_detect_network_pattern() {
        let detector = SysadminPatternDetector::new();
        let result = detector.detect("Configure static IP address on network interface eth0");

        assert!(
            !result.patterns.is_empty(),
            "Should detect at least one pattern"
        );
        let has_network = result
            .patterns
            .iter()
            .any(|p| p.category == SysadminCategory::Networking);
        assert!(has_network, "Should detect networking pattern");
    }

    #[test]
    fn test_detect_os_installation_pattern() {
        let detector = SysadminPatternDetector::new();
        let result = detector.detect("Install Ubuntu 22.04 from ISO");

        assert!(
            !result.patterns.is_empty(),
            "Should detect at least one pattern"
        );
        let has_install = result
            .patterns
            .iter()
            .any(|p| p.category == SysadminCategory::OsInstallation);
        assert!(has_install, "Should detect OS installation pattern");
    }

    #[test]
    fn test_detect_service_pattern() {
        let detector = SysadminPatternDetector::new();
        let result = detector.detect("Start nginx service and enable it at boot using systemctl");

        assert!(
            !result.patterns.is_empty(),
            "Should detect at least one pattern"
        );
        let has_service = result
            .patterns
            .iter()
            .any(|p| p.category == SysadminCategory::ServiceManagement);
        assert!(has_service, "Should detect service management pattern");
    }

    #[test]
    fn test_no_pattern_detected() {
        let detector = SysadminPatternDetector::new();
        let result = detector.detect("Write a function to calculate fibonacci numbers");

        // Should have low overall confidence
        assert!(
            result.overall_confidence < 0.3,
            "Should have low confidence for non-sysadmin task"
        );
    }

    #[test]
    fn test_generate_prompt_augmentation() {
        let detector = SysadminPatternDetector::new();
        let result = detector.detect("Create a QEMU VM with KVM acceleration");

        let augmentation = detector.generate_prompt_augmentation(&result);

        assert!(!augmentation.is_empty(), "Should generate augmentation");
        assert!(augmentation.contains("QEMU"), "Should mention QEMU");
        assert!(
            augmentation.contains("Steps") || augmentation.contains("steps"),
            "Should include steps"
        );
    }

    #[test]
    fn test_multiple_patterns_detected() {
        let detector = SysadminPatternDetector::new();
        let result =
            detector.detect("Install Ubuntu in a QEMU VM and configure static IP networking");

        assert!(
            result.patterns.len() >= 2,
            "Should detect multiple patterns"
        );
    }
}
