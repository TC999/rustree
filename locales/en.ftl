## 英文（默认）语言包 —— rt 命令（rustree）
## 由 fluent 加载；控制字符 \u0008（bold）、\u000C（italic）、\r（endcolor）
## 由 color::fancy 解释，对应原 C 的 \b / \f / \r。

## ---- 错误消息 ----

invalid-option-char = rt: Invalid argument -`{ $char }'.
invalid-option = rt: Invalid argument `{ $arg }'.
missing-option-arg = rt: Missing argument to -{ $opt } option.
invalid-level = rt: Invalid level, must be greater than 0.
missing-long-arg-eq = rt: Missing argument to { $prefix }=
missing-long-arg = rt: Missing argument to { $prefix }
invalid-sort = rt: Sort type '{ $arg }' not valid, should be one of: { $list }
load-gitignore-fail = rt: Could not load gitignore file
load-infofile-fail = rt: Could not load infofile
get-hostname-fail = Unable to get hostname, using 'localhost'.
error-opening-dir = error opening dir
error-opening-file = rt: Error opening { $path } for reading.
invalid-filename = rt: invalid filename '{ $f }'
filelimit-exceeded = { $n } entries exceeds filelimit, not opening dir
recursive-not-followed = recursive, not followed
valid-charsets = Valid charsets include:
report-unit = { " bytes" }

## ---- 目录统计报告（unix_report / html_report）----
## 四种变体：du（--du 时含 size 前缀）与非 du；full（含文件数）与 dirs（-d 时）
## 英文复数：变体值用完整词（directory/directories、file/files）

report-full = { $dirs } { $dirs ->
        [one] directory
       *[other] directories
    }, { $files } { $files ->
        [one] file
       *[other] files
    }
report-full-du = { $size }{ $unit } used in { $dirs } { $dirs ->
        [one] directory
       *[other] directories
    }, { $files } { $files ->
        [one] file
       *[other] files
    }
report-dirs = { $dirs } { $dirs ->
        [one] directory
       *[other] directories
    }
report-dirs-du = { $size }{ $unit } used in { $dirs } { $dirs ->
        [one] directory
       *[other] directories
    }

## ---- HTML 文案 ----

html-title = Directory Tree
html-author = Made by 'rt'

## ---- usage / 帮助文本 ----
## 每行一条独立消息（纯文本，选项名与描述内联）；
## usage-summary 为多行（\u000A 换行、\u0009 缩进）

usage-summary = { "usage: rt [-acdfghilnpqrstuvxACDFJQNUX] [-L level [-R]] [-H [-]baseHREF]\u000A\u0009[-T title] [-o filename] [-P pattern] [-I pattern] [--gitignore]\u000A\u0009[--gitfile[=]file] [--matchdirs] [--metafirst] [--ignore-case]\u000A\u0009[--nolinks] [--hintro[=]file] [--houtro[=]file] [--inodes] [--device]\u000A\u0009[--sort[=]name] [--dirsfirst] [--filesfirst] [--filelimit[=]#] [--si]\u000A\u0009[--du] [--prune] [--timefmt[=]format] [--fromfile]\u000A\u0009[--fromtabfile] [--fflinks] [--info] [--infofile[=]file] [--noreport]\u000A\u0009[--hyperlink] [--scheme[=]schema] [--authority[=]host] [--opt-toggle]\u000A\u0009[--compress[=]#] [--condense] [--version] [--help]\u000A\u0009[--] [directory ...]" }
help-listing-options = { "  ------- Listing options -------" }
help-all-files = { "  -a            All files are listed." }
help-list-dirs-only = { "  -d            List directories only." }
help-follow-symlinks = { "  -l            Follow symbolic links like directories." }
help-print-full-path = { "  -f            Print the full path prefix for each file." }
help-stay-on-fs = { "  -x            Stay on current filesystem only." }
help-descend-level = { "  -L level      Descend only level directories deep." }
help-rerun-tree = { "  -R            Rerun rt when max dir level reached." }
help-list-match-pattern = { "  -P pattern    List only those files that match the pattern given." }
help-exclude-match-pattern = { "  -I pattern    Do not list files that match the given pattern." }
help-filter-gitignore = { "  --gitignore   Filter by using .gitignore files." }
help-explicit-gitfile = { "  --gitfile X   Explicitly read a gitignore file." }
help-ignore-case = { "  --ignore-case Ignore case when pattern matching." }
help-match-dirs = { "  --matchdirs   Include directory names in -P pattern matching." }
help-meta-first = { "  --metafirst   Print meta-data at the beginning of each line." }
help-prune-empty-dirs = { "  --prune       Prune empty directories from the output." }
help-info-files = { "  --info        Print information about files found in .info files." }
help-explicit-infofile = { "  --infofile X  Explicitly read info file." }
help-no-report = { "  --noreport    Turn off file/directory count at end of tree listing." }
help-file-limit = { "  --filelimit # Do not descend dirs with more than # files in them." }
help-condense = { "  --condense    Condense directory singletons to a single line of output." }
help-output-file = { "  -o filename   Output to file instead of stdout." }
help-file-options = { "  ------- File options -------" }
help-print-nonprintable = { "  -q            Print non-printable characters as '?'." }
help-print-raw = { "  -N            Print non-printable characters as is." }
help-quote-filenames = { "  -Q            Quote filenames with double quotes." }
help-print-protections = { "  -p            Print the protections for each file." }
help-display-owner = { "  -u            Displays file owner or UID number." }
help-display-group = { "  -g            Displays file group owner or GID number." }
help-print-size = { "  -s            Print the size in bytes of each file." }
help-human-readable-size = { "  -h            Print the size in a more human readable way." }
help-si-units = { "  --si          Like -h, but use in SI units (powers of 1000)." }
help-compute-dir-size = { "  --du          Compute size of directories by their contents." }
help-print-date = { "  -D            Print the date of last modification or (-c) status change." }
help-time-format = { "  --timefmt fmt Print and format time according to the format fmt." }
help-append-ls = { "  -F            Appends '/', '=', '*', '@', '|' or '>' as per ls -F." }
help-print-inodes = { "  --inodes      Print inode number of each file." }
help-print-device = { "  --device      Print device ID number to which each file belongs." }
help-sorting-options = { "  ------- Sorting options -------" }
help-sort-version = { "  -v            Sort files alphanumerically by version." }
help-sort-mtime = { "  -t            Sort files by last modification time." }
help-sort-ctime = { "  -c            Sort files by last status change time." }
help-unsorted = { "  -U            Leave files unsorted." }
help-reverse-sort = { "  -r            Reverse the order of the sort." }
help-dirs-first = { "  --dirsfirst   List directories before files (-U disables)." }
help-files-first = { "  --filesfirst  List files before directories (-U disables)." }
help-select-sort = { "  --sort X      Select sort: name,version,size,mtime,ctime,none." }
help-graphics-options = { "  ------- Graphics options -------" }
help-no-indent = { "  -i            Don't print indentation lines." }
help-ansi-lines = { "  -A            Print UTF-8 graphic indentation lines." }
help-no-color = { "  -n            Turn colorization off always (-C overrides)." }
help-force-color = { "  -C            Turn colorization on always." }
help-compress-lines = { "  --compress #  Compress indentation lines." }
help-xml-html-options = { "  ------- XML/HTML/JSON/HYPERLINK options -------" }
help-xml-output = { "  -X            Prints out an XML representation of the tree." }
help-json-output = { "  -J            Prints out an JSON representation of the tree." }
help-html-output = { "  -H baseHREF   Prints out HTML format with baseHREF as top directory." }
help-html-title = { "  -T string     Replace the default HTML title and H1 header with string." }
help-no-links = { "  --nolinks     Turn off hyperlinks in HTML output." }
help-html-intro = { "  --hintro X    Use file X as the HTML intro." }
help-html-outro = { "  --houtro X    Use file X as the HTML outro." }
help-hyperlink = { "  --hyperlink   Turn on OSC 8 terminal hyperlinks." }
help-scheme = { "  --scheme X    Set OSC 8 hyperlink scheme, default file://" }
help-authority = { "  --authority X Set OSC 8 hyperlink authority/hostname." }
help-input-options = { "  ------- Input options -------" }
help-from-file = { "  --fromfile    Reads paths from files (.=stdin)" }
help-from-tabfile = { "  --fromtabfile Reads trees from tab indented files (.=stdin)" }
help-fflinks = { "  --fflinks     Process link information when using --fromfile." }
help-misc-options = { "  ------- Miscellaneous options -------" }
help-opt-toggle = { "  --opt-toggle  Enable option toggling." }
help-print-version = { "  --version     Print version and exit." }
help-print-help = { "  --help        Print usage and this help message and exit." }
help-options-terminator = { "  --            Options processing terminator." }
