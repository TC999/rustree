## 简体中文语言包 —— tree 命令（rustree）
## 由 fluent 加载；控制字符 \u0008（bold）、\u000C（italic）、\r（endcolor）
## 由 color::fancy 解释，对应原 C 的 \b / \f / \r。

## ---- 错误消息 ----

invalid-option-char = tree: 无效参数 -`{ $char }'。
invalid-option = tree: 无效参数 `{ $arg }'。
missing-option-arg = tree: 缺少 -{ $opt } 选项的参数。
invalid-level = tree: 无效层级，必须大于 0。
missing-long-arg-eq = tree: 缺少 { $prefix }= 的参数
missing-long-arg = tree: 缺少 { $prefix } 的参数
invalid-sort = tree: 排序类型 '{ $arg }' 无效，应为以下之一：{ $list }
load-gitignore-fail = tree: 无法加载 gitignore 文件
load-infofile-fail = tree: 无法加载 infofile
get-hostname-fail = 无法获取主机名，使用 'localhost'。
error-opening-dir = 打开目录出错
error-opening-file = tree: 打开 { $path } 读取时出错。
invalid-filename = tree: 无效文件名 '{ $f }'
filelimit-exceeded = { $n } 个条目超过 filelimit，不打开目录
recursive-not-followed = 递归，不跟随
valid-charsets = 有效字符集包括：
report-unit = " 字节"

## ---- 目录统计报告（unix_report / html_report）----
## 四种变体：du（--du 时含 size 前缀）与非 du；full（含文件数）与 dirs（-d 时）

report-full = 共 { $dirs } 个目录，{ $files } 个文件
report-full-du = 共 { $dirs } 个目录，{ $files } 个文件，占用 { $size }{ $unit }
report-dirs = 共 { $dirs } 个目录
report-dirs-du = 共 { $dirs } 个目录，占用 { $size }{ $unit }

## ---- HTML 文案 ----

html-title = 目录树
html-author = 由 'tree' 生成

## ---- usage / 帮助文本 ----
## 每行一条独立消息（纯文本，选项名与描述内联）；
## usage-summary 为多行（\u000A 换行、\u0009 缩进）

usage-summary = { "用法：tree [-acdfghilnpqrstuvxACDFJQNUX] [-L 层级 [-R]] [-H [-]基本HREF]\u000A\u0009[-T 标题] [-o 文件名] [-P 模式] [-I 模式] [--gitignore]\u000A\u0009[--gitfile[=]文件] [--matchdirs] [--metafirst] [--ignore-case]\u000A\u0009[--nolinks] [--hintro[=]文件] [--houtro[=]文件] [--inodes] [--device]\u000A\u0009[--sort[=]名称] [--dirsfirst] [--filesfirst] [--filelimit[=]#] [--si]\u000A\u0009[--du] [--prune] [--timefmt[=]格式] [--fromfile]\u000A\u0009[--fromtabfile] [--fflinks] [--info] [--infofile[=]文件] [--noreport]\u000A\u0009[--hyperlink] [--scheme[=]方案] [--authority[=]主机] [--opt-toggle]\u000A\u0009[--compress[=]#] [--condense] [--version] [--help]\u000A\u0009[--] [目录 ...]" }
help-listing-options = { "  ------- 列出选项 -------" }
help-all-files = { "  -a            列出所有文件。" }
help-list-dirs-only = { "  -d            只列出目录。" }
help-follow-symlinks = { "  -l            像目录一样跟随符号链接。" }
help-print-full-path = { "  -f            为每个文件打印完整路径前缀。" }
help-stay-on-fs = { "  -x            仅停留在当前文件系统。" }
help-descend-level = { "  -L 层级      只深入 层级 层目录。" }
help-rerun-tree = { "  -R            到达最大目录层级时重新运行 tree。" }
help-list-match-pattern = { "  -P 模式    只列出与给定模式匹配的文件。" }
help-exclude-match-pattern = { "  -I 模式    不列出与给定模式匹配的文件。" }
help-filter-gitignore = { "  --gitignore   使用 .gitignore 文件过滤。" }
help-explicit-gitfile = { "  --gitfile X   显式读取 gitignore 文件。" }
help-ignore-case = { "  --ignore-case 模式匹配时忽略大小写。" }
help-match-dirs = { "  --matchdirs   在 -P 模式匹配中包含目录名。" }
help-meta-first = { "  --metafirst   在每行开头打印元数据。" }
help-prune-empty-dirs = { "  --prune       从输出中修剪空目录。" }
help-info-files = { "  --info        打印 .info 文件中找到的文件信息。" }
help-explicit-infofile = { "  --infofile X 显式读取信息文件。" }
help-no-report = { "  --noreport    在 tree 列表末尾关闭文件/目录计数。" }
help-file-limit = { "  --filelimit # 不进入超过 # 个文件的目录。" }
help-condense = { "  --condense    将单例目录压缩为单行输出。" }
help-output-file = { "  -o 文件名   输出到文件而非 stdout。" }
help-file-options = { "  ------- 文件选项 -------" }
help-print-nonprintable = { "  -q            将不可打印字符打印为 '?'。" }
help-print-raw = { "  -N            按原样打印不可打印字符。" }
help-quote-filenames = { "  -Q            用双引号引用文件名。" }
help-print-protections = { "  -p            打印每个文件的权限。" }
help-display-owner = { "  -u            显示文件所有者或 UID 号。" }
help-display-group = { "  -g            显示文件组所有者或 GID 号。" }
help-print-size = { "  -s            打印每个文件的字节大小。" }
help-human-readable-size = { "  -h            以更人性化的方式打印大小。" }
help-si-units = { "  --si          类似 -h，但使用 SI 单位（1000 的幂）。" }
help-compute-dir-size = { "  --du          按内容计算目录大小。" }
help-print-date = { "  -D            打印最后修改或 (-c) 状态变更的日期。" }
help-time-format = { "  --timefmt 格式 按 格式 打印和格式化时间。" }
help-append-ls = { "  -F            按 ls -F 追加 '/', '=', '*', '@', '|' 或 '>'。" }
help-print-inodes = { "  --inodes      打印每个文件的 inode 号。" }
help-print-device = { "  --device      打印每个文件所属的设备 ID 号。" }
help-sorting-options = { "  ------- 排序选项 -------" }
help-sort-version = { "  -v            按版本字母数字排序文件。" }
help-sort-mtime = { "  -t            按最后修改时间排序文件。" }
help-sort-ctime = { "  -c            按最后状态变更时间排序文件。" }
help-unsorted = { "  -U            保持文件不排序。" }
help-reverse-sort = { "  -r            反转排序顺序。" }
help-dirs-first = { "  --dirsfirst  先列出目录后列出文件（-U 禁用）。" }
help-files-first = { "  --filesfirst 先列出文件后列出目录（-U 禁用）。" }
help-select-sort = { "  --sort X      选择排序：名称,版本,大小,修改时间,变更时间,无。" }
help-graphics-options = { "  ------- 图形选项 -------" }
help-no-indent = { "  -i            不打印缩进线。" }
help-ansi-lines = { "  -A            打印 UTF-8 图形缩进线。" }
help-no-color = { "  -n            始终关闭彩色化（-C 覆盖）。" }
help-force-color = { "  -C            始终打开彩色化。" }
help-compress-lines = { "  --compress #  压缩缩进线。" }
help-xml-html-options = { "  ------- XML/HTML/JSON/HYPERLINK 选项 -------" }
help-xml-output = { "  -X            打印树的 XML 表示。" }
help-json-output = { "  -J            打印树的 JSON 表示。" }
help-html-output = { "  -H 基本HREF   以 基本HREF 作为顶层目录打印 HTML 格式。" }
help-html-title = { "  -T 字符串    用 字符串 替换默认的 HTML 标题和 H1 头。" }
help-no-links = { "  --nolinks     关闭 HTML 输出中的超链接。" }
help-html-intro = { "  --hintro X    使用文件 X 作为 HTML 简介。" }
help-html-outro = { "  --houtro X    使用文件 X 作为 HTML 结尾。" }
help-hyperlink = { "  --hyperlink   打开 OSC 8 终端超链接。" }
help-scheme = { "  --scheme X    设置 OSC 8 超链接方案，默认 file://" }
help-authority = { "  --authority X 设置 OSC 8 超链接主机/主机名。" }
help-input-options = { "  ------- 输入选项 -------" }
help-from-file = { "  --fromfile    从文件读取路径（.=stdin）" }
help-from-tabfile = { "  --fromtabfile 从制表符缩进的文件读取树（.=stdin）" }
help-fflinks = { "  --fflinks     使用 --fromfile 时处理链接信息。" }
help-misc-options = { "  ------- 其他选项 -------" }
help-opt-toggle = { "  --opt-toggle  启用选项切换。" }
help-print-version = { "  --version     打印版本并退出。" }
help-print-help = { "  --help        打印用法和本帮助信息并退出。" }
help-options-terminator = { "  --            选项处理终止符。" }
