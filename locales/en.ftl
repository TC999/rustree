## 英文（默认）语言包 —— tree 命令（rustree）
## 由 fluent 加载；控制字符 \u0008（bold）、\u000C（italic）、\r（endcolor）
## 由 color::fancy 解释，对应原 C 的 \b / \f / \r。

## ---- 错误消息 ----

invalid-option-char = tree: Invalid argument -`{ $char }'.
invalid-option = tree: Invalid argument `{ $arg }'.
missing-option-arg = tree: Missing argument to -{ $opt } option.
invalid-level = tree: Invalid level, must be greater than 0.
missing-long-arg-eq = tree: Missing argument to { $prefix }=
missing-long-arg = tree: Missing argument to { $prefix }
invalid-sort = tree: Sort type '{ $arg }' not valid, should be one of: { $list }
load-gitignore-fail = tree: Could not load gitignore file
load-infofile-fail = tree: Could not load infofile
get-hostname-fail = Unable to get hostname, using 'localhost'.
error-opening-dir = error opening dir
error-opening-file = tree: Error opening { $path } for reading.
invalid-filename = tree: invalid filename '{ $f }'
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
html-author = Made by 'tree'

## ---- usage / 帮助文本 ----

usage-summary = { "usage: \u0008tree\u000D [\u0008-acdfghilnpqrstuvxACDFJQNUX\u000D] [\u0008-L\u000D \u000Clevel\u000D [\u0008-R\u000D]] [\u0008-H\u000D [-]\u000CbaseHREF\u000D]\u000A\u0009[\u0008-T\u000D \u000Ctitle\u000D] [\u0008-o\u000D \u000Cfilename\u000D] [\u0008-P\u000D \u000Cpattern\u000D] [\u0008-I\u000D \u000Cpattern\u000D] [\u0008--gitignore\u000D]\u000A\u0009[\u0008--gitfile\u000D[\u0008=\u000D]\u000Cfile\u000D] [\u0008--matchdirs\u000D] [\u0008--metafirst\u000D] [\u0008--ignore-case\u000D]\u000A\u0009[\u0008--nolinks\u000D] [\u0008--hintro\u000D[\u0008=\u000D]\u000Cfile\u000D] [\u0008--houtro\u000D[\u0008=\u000D]\u000Cfile\u000D] [\u0008--inodes\u000D] [\u0008--device\u000D]\u000A\u0009[\u0008--sort\u000D[\u0008=\u000D]\u000Cname\u000D] [\u0008--dirsfirst\u000D] [\u0008--filesfirst\u000D] [\u0008--filelimit\u000D[\u0008=\u000D]\u000C#\u000D] [\u0008--si\u000D]\u000A\u0009[\u0008--du\u000D] [\u0008--prune\u000D] [\u0008--timefmt\u000D[\u0008=\u000D]\u000Cformat\u000D] [\u0008--fromfile\u000D]\u000A\u0009[\u0008--fromtabfile\u000D] [\u0008--fflinks\u000D] [\u0008--info\u000D] [\u0008--infofile\u000D[\u0008=\u000D]\u000Cfile\u000D] [\u0008--noreport\u000D]\u000A\u0009[\u0008--hyperlink\u000D] [\u0008--scheme\u000D[\u0008=\u000D]\u000Cschema\u000D] [\u0008--authority\u000D[\u0008=\u000D]\u000Chost\u000D] [\u0008--opt-toggle\u000D]\u000A\u0009[\u0008--compress\u000D[\u0008=\u000D]\u000C#\u000D] [\u0008--condense\u000D] [\u0008--version\u000D] [\u0008--help\u000D]\u000A\u0009[\u0008--\u000D] [\u000Cdirectory\u000D \u0008...\u000D]" }
usage-listing = { "  \u0008------- Listing options -------\u000D\u000A  \u0008-a\u000D            All files are listed.\u000A  \u0008-d\u000D            List directories only.\u000A  \u0008-l\u000D            Follow symbolic links like directories.\u000A  \u0008-f\u000D            Print the full path prefix for each file.\u000A  \u0008-x\u000D            Stay on current filesystem only.\u000A  \u0008-L\u000D \u000Clevel\u000D      Descend only \u000Clevel\u000D directories deep.\u000A  \u0008-R\u000D            Rerun tree when max dir level reached.\u000A  \u0008-P\u000D \u000Cpattern\u000D    List only those files that match the pattern given.\u000A  \u0008-I\u000D \u000Cpattern\u000D    Do not list files that match the given pattern.\u000A  \u0008--gitignore\u000D   Filter by using \u0008.gitignore\u000D files.\u000A  \u0008--gitfile\u000D \u000CX\u000D   Explicitly read a gitignore file.\u000A  \u0008--ignore-case\u000D Ignore case when pattern matching.\u000A  \u0008--matchdirs\u000D   Include directory names in \u0008-P\u000D pattern matching.\u000A  \u0008--metafirst\u000D   Print meta-data at the beginning of each line.\u000A  \u0008--prune\u000D       Prune empty directories from the output.\u000A  \u0008--info\u000D        Print information about files found in \u0008.info\u000D files.\u000A  \u0008--infofile\u000D \u000CX\u000D  Explicitly read info file.\u000A  \u0008--noreport\u000D    Turn off file/directory count at end of tree listing.\u000A  \u0008--filelimit\u000D \u000C#\u000D Do not descend dirs with more than \u000C#\u000D files in them.\u000A  \u0008--condense\u000D    Condense directory singletons to a single line of output.\u000A  \u0008-o\u000D \u000Cfilename\u000D   Output to file instead of stdout." }
usage-file = { "  \u0008------- File options -------\u000D\u000A  \u0008-q\u000D            Print non-printable characters as '\u0008?\u000D'.\u000A  \u0008-N\u000D            Print non-printable characters as is.\u000A  \u0008-Q\u000D            Quote filenames with double quotes.\u000A  \u0008-p\u000D            Print the protections for each file.\u000A  \u0008-u\u000D            Displays file owner or UID number.\u000A  \u0008-g\u000D            Displays file group owner or GID number.\u000A  \u0008-s\u000D            Print the size in bytes of each file.\u000A  \u0008-h\u000D            Print the size in a more human readable way.\u000A  \u0008--si\u000D          Like \u0008-h\u000D, but use in SI units (powers of 1000).\u000A  \u0008--du\u000D          Compute size of directories by their contents.\u000A  \u0008-D\u000D            Print the date of last modification or (-c) status change.\u000A  \u0008--timefmt\u000D \u000Cfmt\u000D Print and format time according to the format \u000Cfmt\u000D.\u000A  \u0008-F\u000D            Appends '\u0008/\u000D', '\u0008=\u000D', '\u0008*\u000D', '\u0008@\u000D', '\u0008|\u000D' or '\u0008>\u000D' as per \u0008ls -F\u000D.\u000A  \u0008--inodes\u000D      Print inode number of each file.\u000A  \u0008--device\u000D      Print device ID number to which each file belongs." }
usage-sorting = { "  \u0008------- Sorting options -------\u000D\u000A  \u0008-v\u000D            Sort files alphanumerically by version.\u000A  \u0008-t\u000D            Sort files by last modification time.\u000A  \u0008-c\u000D            Sort files by last status change time.\u000A  \u0008-U\u000D            Leave files unsorted.\u000A  \u0008-r\u000D            Reverse the order of the sort.\u000A  \u0008--dirsfirst\u000D   List directories before files (\u0008-U\u000D disables).\u000A  \u0008--filesfirst\u000D  List files before directories (\u0008-U\u000D disables).\u000A  \u0008--sort\u000D \u000CX\u000D      Select sort: \u0008\u000Cname\u000D,\u0008\u000Cversion\u000D,\u0008\u000Csize\u000D,\u0008\u000Cmtime\u000D,\u0008\u000Cctime\u000D,\u0008\u000Cnone\u000D." }
usage-graphics = { "  \u0008------- Graphics options -------\u000D\u000A  \u0008-i\u000D            Don't print indentation lines.\u000A  \u0008-A\u000D            Print UTF-8 graphic indentation lines.\u000A  \u0008-n\u000D            Turn colorization off always (\u0008-C\u000D overrides).\u000A  \u0008-C\u000D            Turn colorization on always.\u000A  \u0008--compress\u000D \u000C#\u000D  Compress indentation lines." }
usage-xml-html = { "  \u0008------- XML/HTML/JSON/HYPERLINK options -------\u000D\u000A  \u0008-X\u000D            Prints out an XML representation of the tree.\u000A  \u0008-J\u000D            Prints out an JSON representation of the tree.\u000A  \u0008-H\u000D \u000CbaseHREF\u000D   Prints out HTML format with \u000CbaseHREF\u000D as top directory.\u000A  \u0008-T\u000D \u000Cstring\u000D     Replace the default HTML title and H1 header with \u000Cstring\u000D.\u000A  \u0008--nolinks\u000D     Turn off hyperlinks in HTML output.\u000A  \u0008--hintro\u000D \u000CX\u000D    Use file \u000CX\u000D as the HTML intro.\u000A  \u0008--houtro\u000D \u000CX\u000D    Use file \u000CX\u000D as the HTML outro.\u000A  \u0008--hyperlink\u000D   Turn on OSC 8 terminal hyperlinks.\u000A  \u0008--scheme\u000D \u000CX\u000D    Set OSC 8 hyperlink scheme, default \u0008\u000Cfile://\u000D\u000A  \u0008--authority\u000D \u000CX\u000D Set OSC 8 hyperlink authority/hostname." }
usage-input = { "  \u0008------- Input options -------\u000D\u000A  \u0008--fromfile\u000D    Reads paths from files (\u0008.\u000D=stdin)\u000A  \u0008--fromtabfile\u000D Reads trees from tab indented files (\u0008.\u000D=stdin)\u000A  \u0008--fflinks\u000D     Process link information when using \u0008--fromfile\u000D." }
usage-misc = { "  \u0008------- Miscellaneous options -------\u000D\u000A  \u0008--opt-toggle\u000D  Enable option toggling.\u000A  \u0008--version\u000D     Print version and exit.\u000A  \u0008--help\u000D        Print usage and this help message and exit.\u000A  \u0008--\u000D            Options processing terminator." }