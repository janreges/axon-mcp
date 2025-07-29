#!/bin/bash
# Helper script for agents to check project status

echo "=== PROJECT STATUS CHECK ==="
echo ""

# Check phase completion
echo "PHASE STATUS:"
grep "\[PHASE-.*-COMPLETE\]" STATUS.md 2>/dev/null | tail -5 || echo "  No phases completed yet"
echo ""

# Check crate completion
echo "CRATE STATUS:"
for crate in CORE DATABASE PROTOCOL MOCKS SERVER; do
    if grep -q "\[$crate-COMPLETE\]" STATUS.md 2>/dev/null; then
        echo "  ✓ $crate: Complete"
    elif grep -q "\[$crate-START\]" STATUS.md 2>/dev/null; then
        echo "  ⚡ $crate: In Progress"
    else
        echo "  ⏳ $crate: Not Started"
    fi
done
echo ""

# Check for blockers
echo "BLOCKING ISSUES:"
blockers=$(grep "\[BLOCKED-" STATUS.md 2>/dev/null | grep -v "RESOLVED" | tail -5)
if [ -z "$blockers" ]; then
    echo "  No active blockers"
else
    echo "$blockers"
fi
echo ""

# Check available interfaces
echo "AVAILABLE INTERFACES:"
grep "\[INTERFACE-" INTERFACES.md 2>/dev/null | tail -10 || echo "  No interfaces defined yet"
echo ""

# Recent decisions
echo "RECENT DECISIONS:"
grep "\[DECISION-" DECISIONS.md 2>/dev/null | tail -3 || echo "  No decisions recorded yet"