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

usage-summary = { "用法：\u0008tree\u000D [\u0008-acdfghilnpqrstuvxACDFJQNUX\u000D] [\u0008-L\u000D \u000C层级\u000D [\u0008-R\u000D]] [\u0008-H\u000D [-]\u000C基本HREF\u000D]\u000A\u0009[\u0008-T\u000D \u000C标题\u000D] [\u0008-o\u000D \u000C文件名\u000D] [\u0008-P\u000D \u000C模式\u000D] [\u0008-I\u000D \u000C模式\u000D] [\u0008--gitignore\u000D]\u000A\u0009[\u0008--gitfile\u000D[\u0008=\u000D]\u000C文件\u000D] [\u0008--matchdirs\u000D] [\u0008--metafirst\u000D] [\u0008--ignore-case\u000D]\u000A\u0009[\u0008--nolinks\u000D] [\u0008--hintro\u000D[\u0008=\u000D]\u000C文件\u000D] [\u0008--houtro\u000D[\u0008=\u000D]\u000C文件\u000D] [\u0008--inodes\u000D] [\u0008--device\u000D]\u000A\u0009[\u0008--sort\u000D[\u0008=\u000D]\u000C名称\u000D] [\u0008--dirsfirst\u000D] [\u0008--filesfirst\u000D] [\u0008--filelimit\u000D[\u0008=\u000D]\u000C#\u000D] [\u0008--si\u000D]\u000A\u0009[\u0008--du\u000D] [\u0008--prune\u000D] [\u0008--timefmt\u000D[\u0008=\u000D]\u000C格式\u000D] [\u0008--fromfile\u000D]\u000A\u0009[\u0008--fromtabfile\u000D] [\u0008--fflinks\u000D] [\u0008--info\u000D] [\u0008--infofile\u000D[\u0008=\u000D]\u000C文件\u000D] [\u0008--noreport\u000D]\u000A\u0009[\u0008--hyperlink\u000D] [\u0008--scheme\u000D[\u0008=\u000D]\u000C方案\u000D] [\u0008--authority\u000D[\u0008=\u000D]\u000C主机\u000D] [\u0008--opt-toggle\u000D]\u000A\u0009[\u0008--compress\u000D[\u0008=\u000D]\u000C#\u000D] [\u0008--condense\u000D] [\u0008--version\u000D] [\u0008--help\u000D]\u000A\u0009[\u0008--\u000D] [\u000C目录\u000D \u0008...\u000D]" }
usage-listing = { "  \u0008------- 列出选项 -------\u000D\u000A  \u0008-a\u000D            列出所有文件。\u000A  \u0008-d\u000D            只列出目录。\u000A  \u0008-l\u000D            像目录一样跟随符号链接。\u000A  \u0008-f\u000D            为每个文件打印完整路径前缀。\u000A  \u0008-x\u000D            仅停留在当前文件系统。\u000A  \u0008-L\u000D \u000C层级\u000D      只深入 \u000C层级\u000D 层目录。\u000A  \u0008-R\u000D            到达最大目录层级时重新运行 tree。\u000A  \u0008-P\u000D \u000C模式\u000D    只列出与给定模式匹配的文件。\u000A  \u0008-I\u000D \u000C模式\u000D    不列出与给定模式匹配的文件。\u000A  \u0008--gitignore\u000D   使用 \u0008.gitignore\u000D 文件过滤。\u000A  \u0008--gitfile\u000D \u000CX\u000D   显式读取 gitignore 文件。\u000A  \u0008--ignore-case\u000D 模式匹配时忽略大小写。\u000A  \u0008--matchdirs\u000D   在 \u0008-P\u000D 模式匹配中包含目录名。\u000A  \u0008--metafirst\u000D   在每行开头打印元数据。\u000A  \u0008--prune\u000D       从输出中修剪空目录。\u000A  \u0008--info\u000D        打印 \u0008.info\u000D 文件中找到的文件信息。\u000A  \u0008--infofile\u000D \u000CX\u000D 显式读取信息文件。\u000A  \u0008--noreport\u000D    在 tree 列表末尾关闭文件/目录计数。\u000A  \u0008--filelimit\u000D \u000C#\u000D 不进入超过 \u000C#\u000D 个文件的目录。\u000A  \u0008--condense\u000D    将单例目录压缩为单行输出。\u000A  \u0008-o\u000D \u000C文件名\u000D   输出到文件而非 stdout。" }
usage-file = { "  \u0008------- 文件选项 -------\u000D\u000A  \u0008-q\u000D            将不可打印字符打印为 '\u0008?\u000D'。\u000A  \u0008-N\u000D            按原样打印不可打印字符。\u000A  \u0008-Q\u000D            用双引号引用文件名。\u000A  \u0008-p\u000D            打印每个文件的权限。\u000A  \u0008-u\u000D            显示文件所有者或 UID 号。\u000A  \u0008-g\u000D            显示文件组所有者或 GID 号。\u000A  \u0008-s\u000D            打印每个文件的字节大小。\u000A  \u0008-h\u000D            以更人性化的方式打印大小。\u000A  \u0008--si\u000D          类似 \u0008-h\u000D，但使用 SI 单位（1000 的幂）。\u000A  \u0008--du\u000D          按内容计算目录大小。\u000A  \u0008-D\u000D            打印最后修改或 (-c) 状态变更的日期。\u000A  \u0008--timefmt\u000D \u000C格式\u000D 按 \u000C格式\u000D 打印和格式化时间。\u000A  \u0008-F\u000D            按 \u0008ls -F\u000D 追加 '\u0008/\u000D'、'\u0008=\u000D'、'\u0008*\u000D'、'\u0008@\u000D'、'\u0008|\u000D' 或 '\u0008>\u000D'。\u000A  \u0008--inodes\u000D      打印每个文件的 inode 号。\u000A  \u0008--device\u000D      打印每个文件所属的设备 ID 号。" }
usage-sorting = { "  \u0008------- 排序选项 -------\u000D\u000A  \u0008-v\u000D            按版本字母数字排序文件。\u000A  \u0008-t\u000D            按最后修改时间排序文件。\u000A  \u0008-c\u000D            按最后状态变更时间排序文件。\u000A  \u0008-U\u000D            保持文件不排序。\u000A  \u0008-r\u000D            反转排序顺序。\u000A  \u0008--dirsfirst\u000D  先列出目录后列出文件（\u0008-U\u000D 禁用）。\u000A  \u0008--filesfirst\u000D 先列出文件后列出目录（\u0008-U\u000D 禁用）。\u000A  \u0008--sort\u000D \u000CX\u000D      选择排序：\u0008\u000C名称\u000D,\u0008\u000C版本\u000D,\u0008\u000C大小\u000D,\u0008\u000C修改时间\u000D,\u0008\u000C变更时间\u000D,\u0008\u000C无\u000D." }
usage-graphics = { "  \u0008------- 图形选项 -------\u000D\u000A  \u0008-i\u000D            不打印缩进线。\u000A  \u0008-A\u000D            打印 UTF-8 图形缩进线。\u000A  \u0008-n\u000D            始终关闭彩色化（\u0008-C\u000D 覆盖）。\u000A  \u0008-C\u000D            始终打开彩色化。\u000A  \u0008--compress\u000D \u000C#\u000D  压缩缩进线。" }
usage-xml-html = { "  \u0008------- XML/HTML/JSON/HYPERLINK 选项 -------\u000D\u000A  \u0008-X\u000D            打印树的 XML 表示。\u000A  \u0008-J\u000D            打印树的 JSON 表示。\u000A  \u0008-H\u000D \u000C基本HREF\u000D   以 \u000C基本HREF\u000D 作为顶层目录打印 HTML 格式。\u000A  \u0008-T\u000D \u000C字符串\u000D    用 \u000C字符串\u000D 替换默认的 HTML 标题和 H1 头。\u000A  \u0008--nolinks\u000D     关闭 HTML 输出中的超链接。\u000A  \u0008--hintro\u000D \u000CX\u000D    使用文件 \u000CX\u000D 作为 HTML 简介。\u000A  \u0008--houtro\u000D \u000CX\u000D    使用文件 \u000CX\u000D 作为 HTML 结尾。\u000A  \u0008--hyperlink\u000D   打开 OSC 8 终端超链接。\u000A  \u0008--scheme\u000D \u000CX\u000D    设置 OSC 8 超链接方案，默认 \u0008\u000Cfile://\u000D\u000A  \u0008--authority\u000D \u000CX\u000D 设置 OSC 8 超链接主机/主机名。" }
usage-input = { "  \u0008------- 输入选项 -------\u000D\u000A  \u0008--fromfile\u000D    从文件读取路径（\u0008.\u000D=stdin）\u000A  \u0008--fromtabfile\u000D 从制表符缩进的文件读取树（\u0008.\u000D=stdin）\u000A  \u0008--fflinks\u000D     使用 \u0008--fromfile\u000D 时处理链接信息。" }
usage-misc = { "  \u0008------- 其他选项 -------\u000D\u000A  \u0008--opt-toggle\u000D  启用选项切换。\u000A  \u0008--version\u000D     打印版本并退出。\u000A  \u0008--help\u000D        打印用法和本帮助信息并退出。\u000A  \u0008--\u000D            选项处理终止符。" }