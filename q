[4mRFGREP[24m(1)                                                User Commands                                               [4mRFGREP[24m(1)

[1mNAME[0m
       rfgrep  - A powerful command-line utility for recursively searching and listing files with advanced filtering capabili‐
       ties

[1mSYNOPSIS[0m
       [1mrfgrep [22m[[4mOPTIONS[24m] [[4mPATH[24m] [4mCOMMAND[24m [[4mCOMMAND_OPTIONS[24m]

[1mDESCRIPTION[0m
       [1mrfgrep [22mis a high-performance file search and listing utility written in Rust. It provides advanced search  capabilities
       including  regex  patterns, plain text matching, and whole-word searches. The tool features parallel processing, memory
       mapping for large files, and comprehensive filtering options.

[1mFEATURES[0m
       [1mAdvanced Search[0m
              Regex, plain text, and whole-word matching with context lines

       [1mFile Listing[0m
              Detailed/simple output formats with extension statistics

       [1mPerformance[0m
              Parallel processing with memory mapping for large files

       [1mFiltering[0m
              Extension, size, and binary file filtering

       [1mUtilities[0m
              Clipboard copy, dry-run mode, and progress indicators

[1mGLOBAL OPTIONS[0m
       [1m-v[22m, [1m--verbose[0m
              Enable verbose logging output

       [1m--log [4m[22mFILE[0m
              Write logs to specified file

       [1m--dry-run[0m
              Preview files without processing (useful for testing)

       [1m--max-size [4m[22mSIZE[0m
              Skip files larger than specified MB

       [1m--skip-binary[0m
              Skip binary files (improves performance)

       [1m-h[22m, [1m--help[0m
              Print help information

       [1m-V[22m, [1m--version[0m
              Print version information

[1mCOMMANDS[0m
       [1msearch [22mSearch for patterns in files with advanced filtering

       [1minteractive[0m
              Interactive search mode with real-time filtering

       [1mlist   [22mList files with detailed information and statistics

       [1mcompletions[0m
              Generate shell completion scripts

       [1mhelp   [22mPrint help for a specific command

[1mEXAMPLES[0m
       Search for "HashMap" in Rust files:
              [1mrfgrep search HashMap --extensions rs[0m

       List all Markdown files under 1MB:
              [1mrfgrep list --extensions md --max-size 1[0m

       Search with regex and copy to clipboard:
              [1mrfgrep search fn\s+\w+\s*\( regex --copy[0m

       Recursive search with word boundaries:
              [1mrfgrep search test word --recursive --extensions rs[0m

[1mPERFORMANCE TIPS[0m
       [1mUse --skip-binary[0m
              to avoid unnecessary file checks

       [1mLimit scope[0m
              with --extensions and --max-size

       [1mUse --dry-run first[0m
              to preview files

       [1mEnable --recursive[0m
              for deep directory traversal

[1mENVIRONMENT[0m
       [1mRFGREP_CONFIG[0m
              Path to configuration file (optional)

       [1mRFGREP_LOG_LEVEL[0m
              Set logging level (debug, info, warn, error)

[1mFILES[0m
       [1m~/.config/rfgrep/config.toml[0m
              User configuration file

       [1m~/.cache/rfgrep/[0m
              Cache directory for compiled regex patterns

[1mEXIT STATUS[0m
       [1m0      [22mSuccess

       [1m1      [22mGeneral error

       [1m2      [22mInvalid arguments

       [1m3      [22mFile processing error

[1mBUGS[0m
       Report bugs to: https://github.com/kh3rld/rfgrep/issues

[1mAUTHOR[0m
       Written by the rfgrep development team.

[1mCOPYRIGHT[0m
       Copyright © 2025 rfgrep contributors. License GPLv3+: GNU GPL version 3 or later <https://gnu.org/licenses/gpl.html>.

[1mSEE ALSO[0m
       [1mgrep[22m(1), [1mripgrep[22m(1), [1mfind[22m(1)

[1mNOTES[0m
       This man page documents rfgrep version 0.1.1. For the most up-to-date information, visit  https://github.com/kh3rld/rf‐
       grep

rfgrep v0.1.1                                             August 2025                                                [4mRFGREP[24m(1)
