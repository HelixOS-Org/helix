# =============================================================================
# Helix OS - Branch Protection Configuration
# =============================================================================
#
# This file documents the recommended branch protection settings for GitHub.
# Apply these settings manually in your repository:
#
# Settings → Branches → Add branch protection rule
# =============================================================================

# =============================================================================
# MAIN BRANCH PROTECTION
# =============================================================================
# Branch name pattern: main
#
# ┌─────────────────────────────────────────────────────────────────────────────┐
# │ PROTECTION RULES                                                            │
# ├─────────────────────────────────────────────────────────────────────────────┤
# │                                                                             │
# │ ✅ Require a pull request before merging                                    │
# │    ├── ✅ Require approvals: 1 (or more for team projects)                  │
# │    ├── ✅ Dismiss stale pull request approvals when new commits are pushed  │
# │    ├── ✅ Require review from Code Owners (if CODEOWNERS file exists)       │
# │    └── ✅ Require approval of the most recent reviewable push               │
# │                                                                             │
# │ ✅ Require status checks to pass before merging                             │
# │    ├── ✅ Require branches to be up to date before merging                  │
# │    └── Status checks that must pass:                                        │
# │        ├── ✅ CI Success (ci-success)                                       │
# │        ├── ✅ Format Check (format)                                         │
# │        ├── ✅ Clippy Lint (clippy)                                          │
# │        ├── ✅ Cargo Check (check)                                           │
# │        ├── ✅ Build x86_64 (build-x86_64)                                   │
# │        └── ✅ Unit Tests (test)                                             │
# │                                                                             │
# │ ✅ Require conversation resolution before merging                           │
# │                                                                             │
# │ ✅ Require signed commits (optional but recommended)                        │
# │                                                                             │
# │ ✅ Require linear history (optional - enforces rebasing)                    │
# │                                                                             │
# │ ❌ Do not allow bypassing the above settings                                │
# │    (Ensure even admins follow the rules)                                    │
# │                                                                             │
# │ ✅ Restrict who can push to matching branches                               │
# │    (Only allow merging via Pull Requests)                                   │
# │                                                                             │
# └─────────────────────────────────────────────────────────────────────────────┘

# =============================================================================
# DEVELOP BRANCH PROTECTION
# =============================================================================
# Branch name pattern: develop
#
# Similar to main, but potentially less strict:
# - Require 1 approval (can be self-approved for solo developers)
# - Same status checks required
# - Allow force pushes for rebasing (optional)

# =============================================================================
# FEATURE BRANCH NAMING CONVENTION
# =============================================================================
#
# Recommended branch naming:
#   feature/description    - New features
#   fix/description        - Bug fixes
#   docs/description       - Documentation updates
#   refactor/description   - Code refactoring
#   test/description       - Test additions
#   ci/description         - CI/CD changes
#
# Examples:
#   feature/add-memory-allocator
#   fix/kernel-panic-on-boot
#   docs/update-readme
#   refactor/simplify-scheduler

# =============================================================================
# CODEOWNERS (Optional)
# =============================================================================
# Create a .github/CODEOWNERS file to automatically request reviews:
#
# Example CODEOWNERS content:
# ```
# # Default owners for everything
# *                       @your-username
#
# # Kernel core
# /core/                  @your-username
# /hal/                   @your-username
#
# # Documentation
# /docs/                  @your-username
# *.md                    @your-username
#
# # CI/CD
# /.github/               @your-username
# ```

# =============================================================================
# WORKFLOW PERMISSIONS
# =============================================================================
#
# Settings → Actions → General → Workflow permissions:
#   ✅ Read and write permissions
#   ✅ Allow GitHub Actions to create and approve pull requests

# =============================================================================
# QUICK SETUP COMMANDS
# =============================================================================
#
# Via GitHub CLI (gh):
#
# 1. Install GitHub CLI: https://cli.github.com/
#
# 2. Authenticate:
#    gh auth login
#
# 3. Create branch protection (main):
#    gh api repos/{owner}/{repo}/branches/main/protection \
#      --method PUT \
#      --field required_status_checks='{"strict":true,"contexts":["ci-success","format","clippy","check","build-x86_64","test"]}' \
#      --field enforce_admins=true \
#      --field required_pull_request_reviews='{"required_approving_review_count":1,"dismiss_stale_reviews":true}' \
#      --field restrictions=null
#
# 4. Verify protection:
#    gh api repos/{owner}/{repo}/branches/main/protection

# =============================================================================
# RECOMMENDED WORKFLOW
# =============================================================================
#
# 1. Create feature branch from develop:
#    git checkout develop
#    git pull origin develop
#    git checkout -b feature/my-feature
#
# 2. Make changes and commit:
#    git add .
#    git commit -m "feat: add my feature"
#
# 3. Push and create PR:
#    git push origin feature/my-feature
#    gh pr create --base develop --title "feat: add my feature"
#
# 4. Wait for CI checks to pass
#
# 5. Request review and get approval
#
# 6. Merge via GitHub UI (squash or rebase recommended)
#
# 7. Delete feature branch:
#    git checkout develop
#    git pull origin develop
#    git branch -d feature/my-feature
