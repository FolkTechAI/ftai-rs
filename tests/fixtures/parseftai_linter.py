# SPDX-License-Identifier: Apache-2.0

import re
import argparse
from collections import defaultdict
import sys

# Terminal color codes
RED = "\033[91m"
YELLOW = "\033[93m"
GREEN = "\033[92m"
RESET = "\033[0m"

# Core .ftai v2.0 tags
CORE_TAGS = {
    "@ftai", "@document", "@task", "@config", "@ai", "@schema", "@end",
    "@table", "@section", "@note", "@warning", "@goal", "@tool_call",
    "@memory", "@protocol", "@agent", "@issue", "@prose", "@quoted_tag",
    "@constraints", "@insight", "@recommendations", "@closing_note"
}

BLOCK_TAGS = {"@task", "@config", "@ai", "@agent", "@memory", "@protocol"}

# --- Added Syntax Error Checks ---
def check_line_syntax(line_num, raw_line, errors):
    """Checks a single line for basic syntax errors."""
    stripped_line = raw_line.strip()
    leading_whitespace = raw_line[:-len(stripped_line)] if stripped_line else ""

    # 1. Check for mixed tabs and spaces in leading whitespace
    if '\t' in leading_whitespace and ' ' in leading_whitespace:
        errors.append((line_num, f"Mixed tabs and spaces in indentation."))

    # 2. Basic check for unmatched quotes/markup on the line
    #    (This is a simple check, not a full parse)
    if stripped_line.count('"') % 2 != 0:
         # Ignore if it looks like a quoted tag start
        if not stripped_line.startswith('@"'):
             errors.append((line_num, f"Potentially unmatched double quote (\") on line."))
    # Very basic check for ** or /// without pairs on the same line
    if stripped_line.count('**') % 2 != 0:
         errors.append((line_num, f"Potentially unmatched bold marker (**) on line."))
    if stripped_line.count('///') % 2 != 0:
         errors.append((line_num, f"Potentially unmatched highlight marker (///) on line."))
# --- End Added Syntax Error Checks ---

def parse_ftai_with_lines(filepath):
    syntax_errors = [] # Store syntax errors found during parsing
    lines = []
    expected_fail = False # For @intent fail check

    with open(filepath, 'r') as file:
        all_lines = file.readlines()

        # Check for @intent fail on the first line
        if all_lines and all_lines[0].strip().lower() == "@intent fail":
            expected_fail = True

        # --- Modified line reading to include syntax checks ---
        for i, raw_line in enumerate(all_lines):
            line_num = i + 1
            lines.append(raw_line) # Keep raw lines for potential future use
            check_line_syntax(line_num, raw_line, syntax_errors)
        # --- End modification ---

    # Optional: Long file warning
    if len(lines) > 500:
        print(f"{YELLOW}⚠️  Warning: file exceeds recommended line count (500+){RESET}")

    tag_data = []
    buffer = [] # Buffer now stores tuples of (line_num, raw_line)
    current_tag = None
    tag_start = 0

    for i, raw_line in enumerate(lines):
        line = raw_line.strip()
        line_num = i + 1

        if line.startswith("@"):
            if current_tag:
                # Process the buffer content for the previous tag
                # We need the stripped lines for the tag body representation
                processed_buffer = [(ln, l.strip()) for ln, l in buffer]
                tag_data.append((current_tag, processed_buffer, tag_start))
                buffer = [] # Clear buffer for the new tag
            current_tag = line # Store the full tag line
            tag_start = line_num
        elif line == "---":
            continue # Skip separators
        else:
            # Add non-tag lines (with original spacing) to the buffer
            if current_tag is not None: # Only add to buffer if we are inside a tag block
                 buffer.append((line_num, raw_line))

    # Add the last tag found
    if current_tag:
        processed_buffer = [(ln, l.strip()) for ln, l in buffer]
        tag_data.append((current_tag, processed_buffer, tag_start))

    # Return syntax errors and intent along with parsed data
    return tag_data, syntax_errors, expected_fail

def extract_schema_tags(tag_data):
    required_tags = set()
    optional_tags = set()
    for tag, body, _ in tag_data:
        if tag.startswith("@schema"):
            for _, line in body:
                if line.startswith("required_tags:"):
                    required = re.findall(r'"(.*?)"', line)
                    required_tags.update(required)
                elif line.startswith("optional_tags:"):
                    optional = re.findall(r'"(.*?)"', line)
                    optional_tags.update(optional)
    return required_tags, optional_tags

# --- Modified validate_ftai signature ---
def validate_ftai(tag_data, syntax_errors, expected_fail, soft_mode=False, lenient=False):
    # Combine parsing syntax errors with validation errors
    errors = list(syntax_errors) # Start with syntax errors
    warnings = []
    seen_tags = set()
    quoted_tag_count = 0
    has_ftai = False
    has_document = False

    # Extract valid schema tags
    required_schema_tags, optional_schema_tags = extract_schema_tags(tag_data)
    valid_tags = CORE_TAGS.union(required_schema_tags).union(optional_schema_tags)

    for tag, body, line_num in tag_data:
        tag_clean = tag.split()[0]

        # Check for quoted tag usage
        if tag_clean.startswith('@"'):
            quoted_tag_count += 1
            continue

        # Validate tag usage
        if tag_clean not in valid_tags:
            if lenient:
                warnings.append((line_num, f"Unknown tag: {tag_clean}"))
            else:
                errors.append((line_num, f"Unknown tag: {tag_clean}"))
            continue
        else:
            seen_tags.add(tag_clean)

        if tag_clean == "@ftai":
            if line_num != 1:
                warnings.append((line_num, "`@ftai` should be the first tag in the file."))
            has_ftai = True

        if tag_clean == "@document":
            has_document = True

        # Ensure @end is present for block tags
        if tag_clean in BLOCK_TAGS:
            has_end = False
            for _, subtag_line in body:
                if subtag_line.strip() == "@end":
                    has_end = True
                    break
            if not has_end:
                errors.append((line_num, f"Missing `@end` block terminator for {tag_clean}."))

    # Required tag enforcement
    for req in required_schema_tags:
        if req not in seen_tags:
            errors.append((0, f"Missing required schema tag: {req}"))

    if not has_ftai:
        errors.append((0, "Missing required `@ftai` declaration."))
    if not has_document:
        errors.append((0, "Missing required `@document` block."))

    if quoted_tag_count > 10:
        warnings.append((0, "Excessive use of quoted tags (@\"...\"). Consider defining a schema."))

    # --- Added Check for Test Intent ---
    passed = not errors # Did validation pass (ignoring warnings)?

    if expected_fail and passed:
        if soft_mode:
            warnings.append((0, "File passed validation, but was marked with @intent fail (soft mode active)."))
        else:
            # Add a fatal error if a test marked to fail actually passed strict validation
            errors.append((0, "✗ Test Intent Mismatch: File marked with @intent fail passed validation."))
    elif not expected_fail and not passed and any(msg.startswith("✗ Test Intent Mismatch") for _, msg in errors):
         # This case shouldn't normally happen if logic is right, but prevents double reporting
         pass
    # --- End Added Check for Test Intent ---

    return errors, warnings

def print_report(errors, warnings):
    if errors:
        print(f"{RED}❌ FATAL ERRORS:{RESET}")
        for line, msg in errors:
            print(f"{RED}[Line {line}] {msg}{RESET}")
    if warnings:
        print(f"{YELLOW}⚠️  WARNINGS:{RESET}")
        for line, msg in warnings:
            print(f"{YELLOW}[Line {line}] {msg}{RESET}")
    if not errors:
        print(f"{GREEN}✅ PASS: .ftai document is valid.{RESET}")

# Example usage
if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Validate .ftai files.")
    parser.add_argument("filepath", help="Path to the .ftai file to validate.")
    parser.add_argument("--soft", action="store_true", help="Treat unknown tags as warnings instead of errors.")
    parser.add_argument("--lenient", action="store_true", help="Enable lenient mode for unknown tags")
    color_group = parser.add_mutually_exclusive_group()
    color_group.add_argument('--color', dest='color', action='store_true', default=True, help='Enable ANSI color output (default)')
    color_group.add_argument('--no-color', dest='color', action='store_false', help='Disable ANSI color output')
    args = parser.parse_args()

    # Apply color settings based on args.color
    if not args.color:
        RED = ""
        YELLOW = ""
        GREEN = ""
        RESET = ""

    file_path = args.filepath
    # --- Modified parsing call ---
    parsed_data, syntax_errs, intent_fail = parse_ftai_with_lines(file_path)
    # --- Pass new arguments to validator ---
    errs, warns = validate_ftai(parsed_data, syntax_errs, intent_fail, soft_mode=args.soft, lenient=args.lenient)
    print_report(errs, warns)

    # Exit with nonzero code if errors are present
    if errs:
        sys.exit(1)
    else:
        sys.exit(0)
