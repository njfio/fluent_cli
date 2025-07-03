#!/bin/bash

# Comprehensive Code Quality Check Script for fluent_cli
# Based on recommendations from code review analysis

set -euo pipefail

echo "📊 Starting Code Quality Analysis for fluent_cli"
echo "=============================================="

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

# 1. Check code formatting
echo -e "\n${BLUE}1. Checking code formatting...${NC}"
if cargo fmt -- --check >/dev/null 2>&1; then
    log_pass "Code is properly formatted"
else
    log_issue "Code formatting issues found - run 'cargo fmt'"
fi

# 2. Check for clippy warnings
echo -e "\n${BLUE}2. Running Clippy analysis...${NC}"
if cargo clippy --all-targets --all-features -- -D warnings >/dev/null 2>&1; then
    log_pass "No Clippy warnings found"
else
    log_warning "Clippy warnings found - run 'cargo clippy' for details"
fi

# 3. Check for large functions (>50 lines)
echo -e "\n${BLUE}3. Checking function sizes...${NC}"
LARGE_FUNCTIONS=$(find crates/ -name "*.rs" -exec awk '
    /^[[:space:]]*fn / { 
        func_start = NR; 
        func_name = $0; 
        brace_count = 0; 
        in_function = 1;
    }
    in_function && /{/ { brace_count += gsub(/{/, "") }
    in_function && /}/ { 
        brace_count -= gsub(/}/, "");
        if (brace_count == 0) {
            func_length = NR - func_start + 1;
            if (func_length > 50) {
                print FILENAME ":" func_start ": " func_name " (" func_length " lines)";
            }
            in_function = 0;
        }
    }
' {} \; | wc -l)

if [ "$LARGE_FUNCTIONS" -eq 0 ]; then
    log_pass "All functions are reasonably sized (<50 lines)"
else
    log_warning "$LARGE_FUNCTIONS large functions found (>50 lines)"
fi

# 4. Check for complex modules (>500 lines)
echo -e "\n${BLUE}4. Checking module sizes...${NC}"
LARGE_MODULES=0
for file in $(find crates/ -name "*.rs"); do
    lines=$(wc -l < "$file")
    if [ "$lines" -gt 500 ]; then
        log_warning "Large module: $file ($lines lines)"
        ((LARGE_MODULES++))
    fi
done

if [ "$LARGE_MODULES" -eq 0 ]; then
    log_pass "All modules are reasonably sized (<500 lines)"
fi

# 5. Check for proper documentation
echo -e "\n${BLUE}5. Checking documentation coverage...${NC}"
UNDOCUMENTED_ITEMS=$(grep -r "pub fn\|pub struct\|pub enum\|pub trait" --include="*.rs" crates/ | grep -v "test\|example" | wc -l)
DOCUMENTED_ITEMS=$(grep -r -B1 "pub fn\|pub struct\|pub enum\|pub trait" --include="*.rs" crates/ | grep "///" | wc -l)

if [ "$UNDOCUMENTED_ITEMS" -gt 0 ]; then
    DOC_PERCENTAGE=$((DOCUMENTED_ITEMS * 100 / UNDOCUMENTED_ITEMS))
    if [ "$DOC_PERCENTAGE" -gt 70 ]; then
        log_pass "Good documentation coverage ($DOC_PERCENTAGE%)"
    else
        log_warning "Low documentation coverage ($DOC_PERCENTAGE%) - aim for >70%"
    fi
else
    log_info "No public items found to document"
fi

# 6. Check for proper error handling
echo -e "\n${BLUE}6. Checking error handling patterns...${NC}"
UNWRAP_COUNT=$(grep -r "\.unwrap()" --include="*.rs" crates/ | grep -v "test\|example" | wc -l || echo "0")
EXPECT_COUNT=$(grep -r "\.expect(" --include="*.rs" crates/ | grep -v "test\|example" | wc -l || echo "0")
RESULT_COUNT=$(grep -r "Result<" --include="*.rs" crates/ | wc -l || echo "0")

if [ "$UNWRAP_COUNT" -eq 0 ] && [ "$EXPECT_COUNT" -eq 0 ]; then
    log_pass "No unwrap/expect calls found in main code"
elif [ "$UNWRAP_COUNT" -lt 5 ] && [ "$EXPECT_COUNT" -lt 5 ]; then
    log_warning "Few unwrap/expect calls found ($UNWRAP_COUNT unwrap, $EXPECT_COUNT expect)"
else
    log_issue "Many unwrap/expect calls found ($UNWRAP_COUNT unwrap, $EXPECT_COUNT expect)"
fi

# 7. Check for TODO/FIXME comments
echo -e "\n${BLUE}7. Checking for TODO/FIXME comments...${NC}"
TODO_COUNT=$(grep -r -i "todo\|fixme" --include="*.rs" crates/ | grep -v "test\|example" | wc -l || echo "0")

if [ "$TODO_COUNT" -eq 0 ]; then
    log_pass "No TODO/FIXME comments found"
elif [ "$TODO_COUNT" -lt 5 ]; then
    log_warning "$TODO_COUNT TODO/FIXME comments found"
else
    log_issue "$TODO_COUNT TODO/FIXME comments found - consider addressing"
fi

# 8. Check for dead code
echo -e "\n${BLUE}8. Checking for dead code...${NC}"
if cargo build 2>&1 | grep -q "warning.*never used\|warning.*dead_code"; then
    log_warning "Dead code warnings found - run 'cargo build' for details"
else
    log_pass "No dead code warnings found"
fi

# 9. Check test coverage
echo -e "\n${BLUE}9. Checking test coverage...${NC}"
TEST_FILES=$(find crates/ -name "*test*.rs" -o -name "tests.rs" | wc -l)
SOURCE_FILES=$(find crates/ -name "*.rs" | grep -v test | wc -l)

if [ "$SOURCE_FILES" -gt 0 ]; then
    TEST_RATIO=$((TEST_FILES * 100 / SOURCE_FILES))
    if [ "$TEST_RATIO" -gt 20 ]; then
        log_pass "Good test file ratio ($TEST_RATIO%)"
    else
        log_warning "Low test file ratio ($TEST_RATIO%) - aim for >20%"
    fi
fi

# 10. Check for proper module organization
echo -e "\n${BLUE}10. Checking module organization...${NC}"
if [ -f "crates/fluent-cli/src/commands/mod.rs" ] && [ -f "crates/fluent-cli/src/commands/tests.rs" ]; then
    log_pass "Good module organization with command handlers"
else
    log_warning "Consider improving module organization"
fi

# 11. Check for consistent naming conventions
echo -e "\n${BLUE}11. Checking naming conventions...${NC}"
SNAKE_CASE_VIOLATIONS=$(grep -r "fn [A-Z]\|struct [a-z]\|enum [a-z]" --include="*.rs" crates/ | wc -l || echo "0")

if [ "$SNAKE_CASE_VIOLATIONS" -eq 0 ]; then
    log_pass "Consistent naming conventions"
else
    log_warning "$SNAKE_CASE_VIOLATIONS naming convention violations found"
fi

# 12. Check for proper dependency management
echo -e "\n${BLUE}12. Checking dependency management...${NC}"
UNUSED_DEPS=$(cargo machete 2>/dev/null | grep "unused" | wc -l || echo "0")

if command -v cargo-machete >/dev/null 2>&1; then
    if [ "$UNUSED_DEPS" -eq 0 ]; then
        log_pass "No unused dependencies found"
    else
        log_warning "$UNUSED_DEPS unused dependencies found"
    fi
else
    log_info "cargo-machete not installed - run 'cargo install cargo-machete' to check unused deps"
fi

# 13. Check for proper feature flags
echo -e "\n${BLUE}13. Checking feature flag usage...${NC}"
if grep -r "cfg(feature" --include="*.rs" crates/; then
    log_pass "Feature flags found - good for optional functionality"
else
    log_info "No feature flags detected"
fi

# 14. Check for performance considerations
echo -e "\n${BLUE}14. Checking performance patterns...${NC}"
CLONE_COUNT=$(grep -r "\.clone()" --include="*.rs" crates/ | grep -v "test\|example" | wc -l || echo "0")

if [ "$CLONE_COUNT" -lt 20 ]; then
    log_pass "Reasonable clone usage ($CLONE_COUNT instances)"
else
    log_warning "High clone usage ($CLONE_COUNT instances) - consider optimization"
fi

# 15. Check build time
echo -e "\n${BLUE}15. Checking build performance...${NC}"
BUILD_START=$(date +%s)
if cargo check --quiet >/dev/null 2>&1; then
    BUILD_END=$(date +%s)
    BUILD_TIME=$((BUILD_END - BUILD_START))
    
    if [ "$BUILD_TIME" -lt 30 ]; then
        log_pass "Fast build time (${BUILD_TIME}s)"
    elif [ "$BUILD_TIME" -lt 60 ]; then
        log_warning "Moderate build time (${BUILD_TIME}s)"
    else
        log_issue "Slow build time (${BUILD_TIME}s) - consider optimization"
    fi
else
    log_issue "Build failed - fix compilation errors first"
fi

# Summary
echo -e "\n${BLUE}=============================================="
echo "📊 Code Quality Summary"
echo "=============================================="
echo -e "Checks passed: ${GREEN}$CHECKS_PASSED${NC}"
echo -e "Issues found: ${RED}$ISSUES_FOUND${NC}"

# Calculate quality score
TOTAL_CHECKS=$((CHECKS_PASSED + ISSUES_FOUND))
if [ "$TOTAL_CHECKS" -gt 0 ]; then
    QUALITY_SCORE=$((CHECKS_PASSED * 100 / TOTAL_CHECKS))
    echo -e "Quality score: ${BLUE}$QUALITY_SCORE%${NC}"
    
    if [ "$QUALITY_SCORE" -gt 80 ]; then
        echo -e "\n${GREEN}🎉 Excellent code quality!${NC}"
        exit 0
    elif [ "$QUALITY_SCORE" -gt 60 ]; then
        echo -e "\n${YELLOW}👍 Good code quality - minor improvements possible${NC}"
        exit 0
    else
        echo -e "\n${RED}📈 Code quality needs improvement${NC}"
        exit 1
    fi
else
    echo -e "\n${BLUE}ℹ️  Unable to calculate quality score${NC}"
    exit 0
fi
