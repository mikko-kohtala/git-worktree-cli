# Dependency Report for git-worktree-cli

## Summary
**✅ All dependencies are up to date!**  
**✅ No security vulnerabilities found!**

## Project Information
- **Project**: git-worktree-cli v0.1.0
- **Edition**: Rust 2021
- **Total Dependencies**: 238 crate dependencies (including transitive)

## Direct Dependencies Status

### Runtime Dependencies
| Crate | Current Version | Status |
|-------|----------------|--------|
| async-trait | v0.1.88 | ✅ Up to date |
| chrono | v0.4.41 | ✅ Up to date |
| clap | v4.5.40 | ✅ Up to date |
| clap_complete | v4.5.54 | ✅ Up to date |
| colored | v3.0.0 | ✅ Up to date |
| keyring | v3.6.2 | ✅ Up to date |
| regex | v1.11.1 | ✅ Up to date |
| reqwest | v0.12.22 | ✅ Up to date |
| serde | v1.0.219 | ✅ Up to date |
| serde_json | v1.0.140 | ✅ Up to date |
| serde_yaml | v0.9.34+deprecated | ⚠️ Deprecated (but up to date) |
| tabled | v0.20.0 | ✅ Up to date |
| thiserror | v2.0.12 | ✅ Up to date |
| tokio | v1.46.1 | ✅ Up to date |

### Build Dependencies
| Crate | Current Version | Status |
|-------|----------------|--------|
| clap | v4.5.40 | ✅ Up to date |
| clap_complete | v4.5.54 | ✅ Up to date |

### Development Dependencies
| Crate | Current Version | Status |
|-------|----------------|--------|
| assert_cmd | v2.0.17 | ✅ Up to date |
| predicates | v3.1.3 | ✅ Up to date |
| serial_test | v3.2.0 | ✅ Up to date |
| tempfile | v3.20.0 | ✅ Up to date |

## Security Audit Results
- **Security Advisories Checked**: 787 advisories from RustSec advisory database
- **Vulnerabilities Found**: 0
- **Status**: ✅ **No security vulnerabilities detected**

## Notes and Recommendations

### ⚠️ Deprecated Dependencies
- **serde_yaml v0.9.34+deprecated**: This crate is marked as deprecated. Consider migrating to alternatives like `serde_yml` or other YAML parsing libraries for future maintenance.

### ✅ Overall Assessment
Your Rust project is in excellent condition regarding dependency management:
- All dependencies are using their latest available versions
- No security vulnerabilities present
- Dependency tree is healthy with 238 total crates

### Maintenance Recommendations
1. **Monitor serde_yaml**: Consider planning a migration away from the deprecated `serde_yaml` crate
2. **Regular Updates**: Run `cargo outdated` and `cargo audit` periodically to stay current
3. **Automated Checks**: Consider setting up CI/CD to automatically check for outdated dependencies and security issues

## Tools Used
- `cargo-outdated v0.17.0` - For checking outdated dependencies
- `cargo-audit` - For security vulnerability scanning
- Advisory database from RustSec with 787 security advisories

---
*Report generated on: $(date)*
*Total scan time: < 1 minute*