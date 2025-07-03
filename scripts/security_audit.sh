#!/bin/bash

# Comprehensive Security Audit Script for fluent_cli
# Based on recommendations from code review analysis

set -euo pipefail

echo "🔒 Starting Comprehensive Security Audit for fluent_cli"
echo "=================================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
ISSUES_FOUND=0
CHECKS_PASSED=0

log_issue() {
    echo -e "${RED}❌ ISSUE: $1${NC}"
    ((ISSUES_FOUND++))
}

log_pass() {
    echo -e "${GREEN}✅ PASS: $1${NC}"
    ((CHECKS_PASSED++))
}

log_warning() {
    echo -e "${YELLOW}⚠️  WARNING: $1${NC}"
}

log_info() {
    echo -e "${BLUE}ℹ️  INFO: $1${NC}"
}

# 1. Check for hardcoded secrets and API keys
echo -e "\n${BLUE}1. Checking for hardcoded secrets...${NC}"
if grep -r -i "api[_-]key\|secret\|password\|token" --include="*.rs" --include="*.toml" --include="*.json" crates/ | grep -v "api_key: None\|api_key.*Option\|// TODO\|#.*api"; then
    log_issue "Potential hardcoded secrets found"
else
    log_pass "No hardcoded secrets detected"
fi

# 2. Check for unsafe Rust code
echo -e "\n${BLUE}2. Checking for unsafe Rust code...${NC}"
if grep -r "unsafe" --include="*.rs" crates/; then
    log_warning "Unsafe code blocks found - review for necessity"
else
    log_pass "No unsafe code blocks found"
fi

# 3. Check for unwrap() calls that could panic
echo -e "\n${BLUE}3. Checking for unwrap() calls...${NC}"
UNWRAP_COUNT=$(grep -r "\.unwrap()" --include="*.rs" crates/ | wc -l || echo "0")
if [ "$UNWRAP_COUNT" -gt 0 ]; then
    log_warning "$UNWRAP_COUNT unwrap() calls found - consider using proper error handling"
    grep -r "\.unwrap()" --include="*.rs" crates/ | head -5
else
    log_pass "No unwrap() calls found"
fi

# 4. Check for SQL injection vulnerabilities
echo -e "\n${BLUE}4. Checking for potential SQL injection...${NC}"
if grep -r -i "query.*format!\|query.*&\|execute.*format!" --include="*.rs" crates/; then
    log_issue "Potential SQL injection vulnerabilities found"
else
    log_pass "No obvious SQL injection patterns detected"
fi

# 5. Check for command injection in subprocess calls
echo -e "\n${BLUE}5. Checking for command injection vulnerabilities...${NC}"
if grep -r "Command::new\|subprocess\|system\|exec" --include="*.rs" --include="*.py" crates/ . | grep -v "test\|example"; then
    log_warning "Subprocess execution found - verify input sanitization"
else
    log_pass "No subprocess execution in main code"
fi

# 6. Check file permissions and sensitive files
echo -e "\n${BLUE}6. Checking file permissions...${NC}"
if find . -name "*.key" -o -name "*.pem" -o -name "*.p12" -o -name "*.pfx" 2>/dev/null | head -1; then
    log_issue "Sensitive key files found in repository"
else
    log_pass "No sensitive key files found"
fi

# 7. Check for debug/development code in production
echo -e "\n${BLUE}7. Checking for debug code...${NC}"
if grep -r -i "todo\|fixme\|hack\|debug\|println!" --include="*.rs" crates/ | grep -v "test\|example" | head -5; then
    log_warning "Debug/development code found - review before production"
else
    log_pass "No obvious debug code in production paths"
fi

# 8. Check dependencies for known vulnerabilities
echo -e "\n${BLUE}8. Checking dependencies for vulnerabilities...${NC}"
if command -v cargo-audit >/dev/null 2>&1; then
    if cargo audit; then
        log_pass "No known vulnerabilities in dependencies"
    else
        log_issue "Vulnerabilities found in dependencies"
    fi
else
    log_warning "cargo-audit not installed - run 'cargo install cargo-audit' to check dependencies"
fi

# 9. Check for proper error handling
echo -e "\n${BLUE}9. Checking error handling patterns...${NC}"
RESULT_COUNT=$(grep -r "Result<" --include="*.rs" crates/ | wc -l || echo "0")
ERROR_COUNT=$(grep -r "\.map_err\|\.unwrap_or\|\.unwrap_or_else\|match.*Err" --include="*.rs" crates/ | wc -l || echo "0")

if [ "$RESULT_COUNT" -gt 0 ] && [ "$ERROR_COUNT" -gt 0 ]; then
    log_pass "Good error handling patterns detected"
else
    log_warning "Limited error handling patterns found"
fi

# 10. Check for input validation
echo -e "\n${BLUE}10. Checking input validation...${NC}"
if grep -r "validate\|sanitize\|check.*input" --include="*.rs" crates/; then
    log_pass "Input validation code found"
else
    log_warning "Limited input validation detected"
fi

# 11. Check for secure random number generation
echo -e "\n${BLUE}11. Checking random number generation...${NC}"
if grep -r "rand::" --include="*.rs" crates/; then
    if grep -r "thread_rng\|OsRng" --include="*.rs" crates/; then
        log_pass "Secure random number generation found"
    else
        log_warning "Random number generation found - verify cryptographic security"
    fi
else
    log_info "No random number generation detected"
fi

# 12. Check for proper logging (no sensitive data)
echo -e "\n${BLUE}12. Checking logging practices...${NC}"
if grep -r "log::\|println!\|eprintln!" --include="*.rs" crates/ | grep -i "password\|key\|secret\|token"; then
    log_issue "Potential sensitive data in logs"
else
    log_pass "No obvious sensitive data in logging"
fi

# 13. Check for proper configuration management
echo -e "\n${BLUE}13. Checking configuration security...${NC}"
if [ -f "config.yaml" ] || [ -f "config.json" ]; then
    log_warning "Configuration files found in root - ensure they don't contain secrets"
fi

if grep -r "env::" --include="*.rs" crates/; then
    log_pass "Environment variable usage found - good for secrets management"
fi

# 14. Check for proper network security
echo -e "\n${BLUE}14. Checking network security...${NC}"
if grep -r "reqwest\|hyper\|tokio.*net" --include="*.rs" crates/; then
    if grep -r "https\|tls\|ssl" --include="*.rs" crates/; then
        log_pass "HTTPS/TLS usage detected"
    else
        log_warning "Network code found - verify HTTPS/TLS usage"
    fi
fi

# 15. Check for proper memory management
echo -e "\n${BLUE}15. Checking memory management...${NC}"
if grep -r "Box::leak\|mem::forget\|ManuallyDrop" --include="*.rs" crates/; then
    log_warning "Manual memory management found - review for safety"
else
    log_pass "No manual memory management detected"
fi

# Summary
echo -e "\n${BLUE}=================================================="
echo "🔒 Security Audit Summary"
echo "=================================================="
echo -e "Checks passed: ${GREEN}$CHECKS_PASSED${NC}"
echo -e "Issues found: ${RED}$ISSUES_FOUND${NC}"

if [ "$ISSUES_FOUND" -eq 0 ]; then
    echo -e "\n${GREEN}🎉 No critical security issues found!${NC}"
    exit 0
elif [ "$ISSUES_FOUND" -lt 3 ]; then
    echo -e "\n${YELLOW}⚠️  Minor security issues found - review recommended${NC}"
    exit 1
else
    echo -e "\n${RED}🚨 Multiple security issues found - immediate review required${NC}"
    exit 2
fi
