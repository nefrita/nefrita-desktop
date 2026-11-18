BEGIN {
    guide_sections_line_count = split(ENVIRON["GUIDE_SECTIONS"], guide_sections_lines, "\n")
    for (line_position = 1; line_position <= guide_sections_line_count; line_position++) {
        if (guide_sections_lines[line_position] != "") {
            requested_section_titles[guide_sections_lines[line_position]] = 1
        }
    }
}

!inside_fenced_block && /^## / {
    section_title = substr($0, 4)
    next
}

!inside_fenced_block && /^```/ {
    fence_language = substr($0, 4)
    if (fence_language != "bash" && fence_language != "text") {
        print "the section " section_title " of the guide has a code block that is neither bash nor text" > "/dev/stderr"
        found_unexpected_fence_language = 1
        exit 1
    }
    inside_fenced_block = 1
    inside_bash_block = (fence_language == "bash")
    if (inside_bash_block && !(section_title in sections_with_bash_blocks)) {
        sections_with_bash_blocks[section_title] = 1
        if (list_sections_with_bash_blocks) {
            print section_title
        }
    }
    next
}

inside_fenced_block && /^```$/ {
    inside_fenced_block = 0
    inside_bash_block = 0
    next
}

inside_bash_block && !list_sections_with_bash_blocks && (section_title in requested_section_titles) {
    print
}

END {
    if (found_unexpected_fence_language) {
        exit 1
    }
    if (list_sections_with_bash_blocks) {
        exit 0
    }
    requested_section_count = 0
    for (requested_section_title in requested_section_titles) {
        requested_section_count++
        if (!(requested_section_title in sections_with_bash_blocks)) {
            print "the guide has no bash block under the section " requested_section_title > "/dev/stderr"
            exit 1
        }
    }
    if (requested_section_count == 0) {
        print "GUIDE_SECTIONS names no section of the guide" > "/dev/stderr"
        exit 1
    }
}
