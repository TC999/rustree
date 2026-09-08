# 繁體中文（台灣）語言包 —— rt 命令（rustree）
# 控制字元 \b（bold）、\f（italic）、\r（endcolor）由 color::fancy 解譯，對應原 C 的 \b / \f / \r。

# ---- 錯誤訊息 ----

invalid-option-char = rt: 無效的引數 -`{char}'。
invalid-option = rt: 無效的引數 `{arg}'。
missing-option-arg = rt: -{opt} 選項缺少引數。
invalid-level = rt: 無效的層級，必須大於 0。
missing-long-arg-eq = rt: {prefix}= 缺少引數
missing-long-arg = rt: {prefix} 缺少引數
invalid-sort = rt: 排序類型 '{arg}' 無效，應為以下之一：{list}
load-gitignore-fail = rt: 無法載入 gitignore 檔案
load-infofile-fail = rt: 無法載入資訊檔案
get-hostname-fail = 無法取得主機名稱，將使用 'localhost'。
error-opening-dir = 開啟目錄時發生錯誤
error-opening-file = rt: 開啟 {path} 進行讀取時發生錯誤。
invalid-filename = rt: 無效的檔案名稱 '{f}'
filelimit-exceeded = {n} 個項目超過檔案數量限制，不開啟此目錄
recursive-not-followed = 遞迴，未跟隨
valid-charsets = 有效的字元集包括：
report-unit =  bytes

# ---- 目錄統計報告（unix_report / html_report）----
# 四種變體：du（--du 時含 size 前綴）與非 du；full（含檔案數）與 dirs（-d 時）
# 繁體中文無複數變化，統一使用同一形式

report-full = {dirs} 個目錄，{files} 個檔案
report-full-du = 已使用 {size}{unit}，共 {dirs} 個目錄、{files} 個檔案
report-dirs = {dirs} 個目錄
report-dirs-du = 已使用 {size}{unit}，共 {dirs} 個目錄

# ---- HTML 文案 ----

html-title = 目錄樹
html-author = 由 'rt' 產生

# ---- usage / 說明文字 ----
# 每行一條獨立訊息（純文字，選項名與描述內聯）；
# usage-summary 為多行（\n 換行、\t 縮排）

usage-summary = usage: rt [-acdfghilnpqrstuvxACDFJQNUX] [-L level [-R]] [-H [-]baseHREF]\n\t[-T title] [-o filename] [-P pattern] [-I pattern] [--gitignore]\n\t[--gitfile[=]file] [--matchdirs] [--metafirst] [--ignore-case]\n\t[--nolinks] [--hintro[=]file] [--houtro[=]file] [--inodes] [--device]\n\t[--sort[=]name] [--dirsfirst] [--filesfirst] [--filelimit[=]#] [--si]\n\t[--du] [--prune] [--timefmt[=]format] [--fromfile]\n\t[--fromtabfile] [--fflinks] [--info] [--infofile[=]file] [--noreport]\n\t[--hyperlink] [--scheme[=]schema] [--authority[=]host] [--opt-toggle]\n\t[--compress[=]#] [--condense] [--version] [--help]\n\t[--] [directory ...]
help-listing-options =   ------- 列表選項 -------
help-all-files =   -a            列出所有檔案。
help-list-dirs-only =   -d            僅列出目錄。
help-follow-symlinks =   -l            將符號連結視為目錄並跟隨進入。
help-print-full-path =   -f            為每個檔案列印完整路徑前綴。
help-stay-on-fs =   -x            僅停留在目前的檔案系統上。
help-descend-level =   -L level      僅向下遞迴至指定層級的目錄。
help-rerun-tree =   -R            達到最大目錄層級時重新執行 rt。
help-list-match-pattern =   -P pattern    僅列出符合指定模式的檔案。
help-exclude-match-pattern =   -I pattern    不列出符合指定模式的檔案。
help-filter-gitignore =   --gitignore   使用 .gitignore 檔案進行篩選。
help-explicit-gitfile =   --gitfile X   明確指定讀取某個 gitignore 檔案。
help-ignore-case =   --ignore-case 模式比對時忽略大小寫。
help-match-dirs =   --matchdirs   在 -P 模式比對中包含目錄名稱。
help-meta-first =   --metafirst   在每行開頭列印中繼資料。
help-prune-empty-dirs =   --prune       從輸出中移除空目錄。
help-info-files =   --info        列印 .info 檔案中的檔案相關資訊。
help-explicit-infofile =   --infofile X  明確指定讀取某個資訊檔案。
help-no-report =   --noreport    關閉目錄樹列表末尾的檔案／目錄計數。
help-file-limit =   --filelimit # 不遞迴進入檔案數超過 # 的目錄。
help-condense =   --condense    將單一子項目的目錄壓縮為單行輸出。
help-output-file =   -o filename   輸出至檔案而非標準輸出。
help-file-options =   ------- 檔案選項 -------
help-print-nonprintable =   -q            將不可列印字元顯示為 '?'。
help-print-raw =   -N            原樣輸出不可列印字元。
help-quote-filenames =   -Q            以雙引號括住檔案名稱。
help-print-protections =   -p            列印每個檔案的權限。
help-display-owner =   -u            顯示檔案擁有者或 UID 編號。
help-display-group =   -g            顯示檔案群組擁有者或 GID 編號。
help-print-size =   -s            列印每個檔案的位元組大小。
help-human-readable-size =   -h            以較易讀的方式列印檔案大小。
help-si-units =   --si          類似 -h，但使用 SI 單位（1000 的冪次）。
help-compute-dir-size =   --du          依目錄內容計算目錄大小。
help-print-date =   -D            列印最後修改時間或（-c）狀態變更時間。
help-time-format =   --timefmt fmt 依指定格式 fmt 列印並格式化時間。
help-append-ls =   -F            依 ls -F 規則附加 '/'、'='、'*'、'@'、'|' 或 '>'。
help-print-inodes =   --inodes      列印每個檔案的 inode 編號。
help-print-device =   --device      列印每個檔案所屬的裝置 ID 編號。
help-sorting-options =   ------- 排序選項 -------
help-sort-version =   -v            依版本號對檔案進行字母數字排序。
help-sort-mtime =   -t            依最後修改時間排序檔案。
help-sort-ctime =   -c            依最後狀態變更時間排序檔案。
help-unsorted =   -U            不對檔案排序。
help-reverse-sort =   -r            反轉排序順序。
help-dirs-first =   --dirsfirst   目錄列於檔案之前（-U 可停用）。
help-files-first =   --filesfirst  檔案列於目錄之前（-U 可停用）。
help-select-sort =   --sort X      選擇排序方式：name、version、size、mtime、ctime、none。
help-graphics-options =   ------- 圖形選項 -------
help-no-indent =   -i            不列印縮排連接線。
help-ansi-lines =   -A            列印 UTF-8 圖形縮排連接線。
help-no-color =   -n            永遠關閉色彩顯示（可被 -C 覆蓋）。
help-force-color =   -C            永遠開啟色彩顯示。
help-compress-lines =   --compress #  壓縮縮排連接線。
help-xml-html-options =   ------- XML／HTML／JSON／超連結選項 -------
help-xml-output =   -X            以 XML 格式輸出目錄樹。
help-json-output =   -J            以 JSON 格式輸出目錄樹。
help-html-output =   -H baseHREF   以 HTML 格式輸出，並以 baseHREF 作為頂層目錄。
help-html-title =   -T string     以 string 取代預設的 HTML 標題與 H1 標頭。
help-no-links =   --nolinks     關閉 HTML 輸出中的超連結。
help-html-intro =   --hintro X    使用檔案 X 作為 HTML 的開頭內容。
help-html-outro =   --houtro X    使用檔案 X 作為 HTML 的結尾內容。
help-hyperlink =   --hyperlink   開啟 OSC 8 終端機超連結。
help-scheme =   --scheme X    設定 OSC 8 超連結協定，預設為 file://
help-authority =   --authority X 設定 OSC 8 超連結的授權單位／主機名稱。
help-input-options =   ------- 輸入選項 -------
help-from-file =   --fromfile    從檔案讀取路徑（.=stdin）
help-from-tabfile =   --fromtabfile 從 Tab 縮排檔案讀取目錄樹（.=stdin）
help-fflinks =   --fflinks     使用 --fromfile 時處理連結資訊。
help-misc-options =   ------- 其他選項 -------
help-opt-toggle =   --opt-toggle  啟用選項切換功能。
help-print-version =   --version     列印版本資訊並結束。
help-print-help =   --help        列印使用方式與本說明訊息並結束。
help-options-terminator =   --            選項處理終止符。