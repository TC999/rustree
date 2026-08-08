// 文件路径：src/list.rs
// 对应 C 源文件：list.c
// 目录树遍历核心：emit_tree（顶层入口）与 listdir（递归），
// 通过全局回调集合 lc 驱动不同输出格式（终端/HTML/XML/JSON）。

use crate::filter::{flush_filterstack, pop_filterstack};
use crate::globals::{DIRS, ERRORS, FLAG, GETFULLTREE, LC, LEVEL, OUTFILE, TOPSORT};
use crate::hash::{findino, saveino};
use crate::info::pop_infostack;
use crate::tree::{lstat_fields, Ignorefile, Info, Infofile, ListingCalls, Totals};
use crate::{push_files, read_dir, setoutput, stat2info};

// C: 获取当前输出回调集合（对应 extern struct listingcalls lc）
// ListingCalls 为 Copy（仅含函数指针），每次读取复制一份
fn lc() -> ListingCalls {
    // unsafe：读取全局回调集合 LC（main 中初始化）
    unsafe { LC.expect("lc 未初始化") }
}

// === 原 C 函数：void null_intro(void) ===
pub fn null_intro() {}

// === 原 C 函数：void null_outtro(void) ===
pub fn null_outtro() {}

// === 原 C 函数：void null_close(struct _info *file, int level, int needcomma) ===
pub fn null_close(_file: Option<&Info>, _level: i32, _needcomma: bool) {}

// === 原 C 函数：void emit_tree(char **dirname, bool needfulltree) ===
/// 遍历 dirname 中的每个顶层目录并输出。
/// dirname 需要 &mut（flag.f 时会被就地去掉尾部 '/'）。
pub fn emit_tree(dirname: &mut [String], needfulltree: bool) {
    let mut tot = Totals::default();
    let mut ig: Option<Box<Ignorefile>> = None;
    let mut inf: Option<Box<Infofile>> = None;
    // C 中 dir/info 初始为 NULL，循环内 lstat 前重置
    let mut dir: Option<Vec<Info>>;
    let mut info: Option<Info>;

    // C: lc.intro();
    (lc().intro)();

    // C: for(i=0; dirname[i]; i++)
    let mut i = 0;
    while i < dirname.len() {
        // unsafe：读写全局 FLAG/REALBASEPATH/DIRPATHOFFSET/HTMLDIRLEN
        unsafe {
            // C: if (flag.hyper) { realpath 处理 }
            if FLAG.hyper {
                match std::fs::canonicalize(&dirname[i]) {
                    // C: if (realpath(dirname[i], realbasepath) == NULL) { realbasepath[0]='\0'; dirpathoffset = 0; }
                    Err(_) => {
                        crate::globals::REALBASEPATH.clear();
                        crate::globals::DIRPATHOFFSET = 0;
                    }
                    // C: else dirpathoffset = strlen(dirname[i]);
                    Ok(rp) => {
                        crate::globals::REALBASEPATH = rp.to_string_lossy().into_owned();
                        crate::globals::DIRPATHOFFSET = dirname[i].len();
                    }
                }
            }

            // C: if (flag.f) { 去掉 dirname[i] 尾部的 '/' }
            if FLAG.f {
                let bytes = dirname[i].as_bytes();
                let mut j = bytes.len();
                // C: do { if (j > 1 && dirname[i][j-1] == '/') dirname[i][--j] = 0;
                //    } while (j > 1 && dirname[i][j-1] == '/');
                while j > 1 && bytes[j - 1] == b'/' {
                    j -= 1;
                }
                dirname[i].truncate(j);
            }
            // C: if (flag.H) htmldirlen = strlen(dirname[i]);
            if FLAG.H {
                crate::globals::HTMLDIRLEN = dirname[i].len();
            }
        }

        // C: if ((n = lstat(dirname[i], &st)) >= 0)
        info = None;
        dir = None;
        // C: ssize_t n（lstat 失败时为 -1；此处先初始化以满足借用检查，随后各分支覆盖）
        let mut n: i64 = 0;
        let mut dev: u64 = 0;
        match lstat_fields(&dirname[i]) {
            // C: else info = NULL;（n 保持 lstat 的负返回值）
            Err(_) => {
                n = -1;
            }
            Ok(st) => {
                dev = st.dev;
                saveino(st.inode, st.dev);
                let mut ent = stat2info(&st);
                // C: info->name = "";
                ent.name = String::new();
                info = Some(ent);
                if needfulltree {
                    // C: dir = getfulltree(dirname[i], 0, st.st_dev, &(info->size), &err);
                    //     n = err? -1 : 0;
                    let mut err: Option<String> = None;
                    let f = unsafe { GETFULLTREE.expect("getfulltree 未初始化") };
                    dir = f(
                        &dirname[i],
                        0,
                        st.dev,
                        &mut info.as_mut().unwrap().size,
                        &mut err,
                    );
                    n = if err.is_some() { -1 } else { 0 };
                } else {
                    // C: push_files(dirname[i], &ig, &inf, true);
                    push_files(&dirname[i], &mut ig, &mut inf, true);
                    // C: dir = read_dir(dirname[i], &n, inf != NULL);
                    dir = read_dir(&dirname[i], &mut n, if inf.is_some() { 1 } else { 0 });
                }
            }
        }

        // C: lc.printinfo(dirname[i], info, 0);
        (lc().printinfo)(&dirname[i], info.as_mut(), 0);

        // C: needsclosed = lc.printfile(dirname[i], dirname[i], info, (dir != NULL) || (!dir && n));
        let needsclosed = (lc().printfile)(
            &dirname[i],
            &dirname[i],
            info.as_ref(),
            if dir.is_some() || (!dir.is_some() && n != 0) {
                1
            } else {
                0
            },
        );
        // C: subtotal = (struct totals){0, 0, 0};
        let mut subtotal = Totals::default();

        // unsafe：读写全局 FLAG/ERRORS
        unsafe {
            // C: if (!dir && n) —— 打开失败
            if dir.is_none() && n != 0 {
                (lc().error)("error opening dir");
                (lc().newline)(info.as_ref(), 0, 0, i + 1 < dirname.len());
                if info.is_none() {
                    ERRORS += 1;
                } else {
                    subtotal.files += 1;
                }
            } else if FLAG.flimit > 0 && n > FLAG.flimit as i64 {
                // C: sprintf(errbuf, "%ld entries exceeds filelimit, not opening dir", n);
                (lc().error)(&format!("{} entries exceeds filelimit, not opening dir", n));
                (lc().newline)(info.as_ref(), 0, 0, i + 1 < dirname.len());
                subtotal.dirs += 1;
            } else {
                (lc().newline)(info.as_ref(), 0, 0, false);
                // C: if (dir) { subtotal = listdir(...); subtotal.dirs++; }
                if let Some(d) = dir.take() {
                    subtotal = listdir(&dirname[i], d, 1, dev, needfulltree);
                    subtotal.dirs += 1;
                }
            }
        }
        // C: if (dir) { free_dir(dir); dir = NULL; } —— Rust 中 dir 已被 listdir 消费

        // C: if (needsclosed) lc.close(info, 0, dirname[i+1] != NULL);
        if needsclosed != 0 {
            (lc().close)(info.as_ref(), 0, i + 1 < dirname.len());
        }

        tot.files += subtotal.files;
        tot.dirs += subtotal.dirs;
        // 不在 listdir 中累计 tot.size——这已在 getfulltree() 中完成：
        // C: if (flag.du) tot.size += info? info->size : 0;
        if unsafe { FLAG.du } {
            tot.size += info.as_ref().map(|i| i.size).unwrap_or(0);
        }

        // C: if (ig != NULL) ig = flush_filterstack();
        if ig.is_some() {
            flush_filterstack();
            ig = None;
        }
        // C: if (inf != NULL) inf = pop_infostack();
        if inf.is_some() {
            pop_infostack();
            inf = None;
        }
        i += 1;
    }

    // C: if (!flag.noreport) lc.report(tot);
    if !unsafe { FLAG.noreport } {
        (lc().report)(tot);
    }
    // C: lc.outtro();
    (lc().outtro)();
}

// === 原 C 函数：struct totals listdir(char *dirname, struct _info **dir, int lev, dev_t dev, bool hasfulltree) ===
/// 递归遍历目录 dir 的内容并输出，返回统计信息。
/// C 中 dir 为 NULL 结尾数组、由调用者释放；Rust 中为 Vec<Info>（值传递，
/// 函数结束即释放，等价于调用者的 free_dir）。
pub fn listdir(dirname: &str, mut dir: Vec<Info>, lev: i32, dev: u64, hasfulltree: bool) -> Totals {
    let mut tot = Totals::default();
    let mut ig: Option<Box<Ignorefile>> = None;
    let mut inf: Option<Box<Infofile>> = None;
    let mut subdir: Option<Vec<Info>> = None;
    // C: int descend, htmldescend = 0;
    let mut htmldescend = 0;
    let mut n: i64 = 0;

    // C: int es = (dirname[strlen(dirname) - 1] == '/');
    let es = dirname.ends_with('/');

    // C: if (dir == NULL || *dir == NULL) return tot;（空目录）
    if dir.is_empty() {
        return tot;
    }

    // C: for(n=0; dir[n]; n++); if (topsort) qsort(dir, n, sizeof(...), topsort);
    if let Some(f) = unsafe { TOPSORT } {
        // C 的 qsort 是不稳定排序 → sort_unstable_by
        dir.sort_unstable_by(|a, b| f(a, b).cmp(&0));
    }

    // C: dirs[lev] = *(dir+1)? 1 : 2;
    unsafe { DIRS[lev as usize] = if dir.len() > 1 { 1 } else { 2 } };

    // C: for (; *dir != NULL; dir++)
    let mut idx = 0;
    while idx < dir.len() {
        // C: lc.printinfo(dirname, *dir, lev);
        // C: lc.printinfo(dirname, *dir, lev);（*dir 非 NULL → Some）
        (lc().printinfo)(dirname, Some(&mut dir[idx]), lev);

        // C: namelen/namemax 检查与 path 构建（Rust 的 String 动态分配，无需 xrealloc）
        let path = if es {
            format!("{}{}", dirname, dir[idx].name)
        } else {
            format!("{}/{}", dirname, dir[idx].name)
        };
        // C: if (flag.f) filename = path; else filename = (*dir)->name;
        let filename = if unsafe { FLAG.f } {
            path.clone()
        } else {
            dir[idx].name.clone()
        };

        // C: descend = 0; err = NULL; newpath = path;
        let mut descend = 0;
        let mut err: Option<String> = None;
        let newpath = path;

        // C: if ((*dir)->isdir)
        if dir[idx].isdir {
            tot.dirs += 1;
            // C: if (flag.condense_singletons) tot.dirs += (*dir)->condensed;
            if unsafe { FLAG.condense_singletons } {
                tot.dirs += dir[idx].condensed;
            }

            // C: if (!hasfulltree) { found = findino(...); if (!found) saveino(...); }
            //     else found = false;
            let found;
            if !hasfulltree {
                found = findino(dir[idx].inode, dir[idx].dev);
                if !found {
                    saveino(dir[idx].inode, dir[idx].dev);
                }
            } else {
                found = false;
            }

            let lnk_is_some = dir[idx].lnk.is_some();
            // C: if (!(flag.xdev && dev != (*dir)->dev) && (!(*dir)->lnk || ((*dir)->lnk && flag.l)))
            if !(unsafe { FLAG.xdev } && dev != dir[idx].dev)
                && (!lnk_is_some || (lnk_is_some && unsafe { FLAG.l }))
            {
                descend = 1;

                // C: if ((*dir)->lnk && found) { err = "recursive, not followed"; ... descend = -1; }
                if lnk_is_some && found {
                    // C: if (Level >= 0 && lev > Level) err = NULL;
                    if unsafe { LEVEL } >= 0 && lev as i64 > unsafe { LEVEL } {
                        // err 保持 NULL
                    } else {
                        err = Some("recursive, not followed".to_string());
                    }
                    descend = -1;
                }

                // C: if ((Level >= 0) && (lev > Level))
                if unsafe { LEVEL } >= 0 && lev as i64 > unsafe { LEVEL } {
                    // C: if (flag.R) —— HTML 模式为子目录生成 00Tree.html
                    if unsafe { FLAG.R } {
                        // C: FILE *outsave = outfile;
                        let outsave = unsafe { OUTFILE.take() };
                        // C: char *paths[2] = {newpath, NULL};
                        let mut paths = vec![newpath.clone()];
                        // C: sprintf(output, "%s/00Tree.html", newpath);
                        let output = format!("{}/00Tree.html", newpath);
                        // C: memcpy(dirsave, dirs, sizeof(int) * (lev+1));
                        let dirsave: Vec<i32> = unsafe { DIRS[..lev as usize + 1].to_vec() };
                        // C: setoutput(output); emit_tree(paths, hasfulltree);
                        setoutput(Some(&output));
                        emit_tree(&mut paths, hasfulltree);
                        // C: fclose(outfile); outfile = outsave;
                        unsafe {
                            OUTFILE = outsave;
                        }
                        // C: memcpy(dirs, dirsave, sizeof(int) * (lev+1));
                        unsafe {
                            for (k, &v) in dirsave.iter().enumerate() {
                                if k < DIRS.len() {
                                    DIRS[k] = v;
                                }
                            }
                        }
                        htmldescend = 10;
                    } else {
                        htmldescend = 0;
                    }
                    descend = 0;
                }

                // C: if (descend > 0)
                if descend > 0 {
                    if hasfulltree {
                        // C: subdir = (*dir)->child; err = (*dir)->err;
                        subdir = dir[idx].child.take();
                        err = dir[idx].err.take();
                    } else {
                        // C: push_files(newpath, &ig, &inf, false);
                        push_files(&newpath, &mut ig, &mut inf, false);
                        // C: subdir = read_dir(newpath, &n, inf != NULL);
                        subdir = read_dir(&newpath, &mut n, if inf.is_some() { 1 } else { 0 });
                        // C: if (!subdir && n) { err = "error opening dir"; errors++; }
                        if subdir.is_none() && n != 0 {
                            err = Some("error opening dir".to_string());
                            unsafe { ERRORS += 1; }
                        }
                        // C: if (flag.flimit > 0 && n > flag.flimit) { ... errors++; free_dir(subdir); subdir = NULL; }
                        if unsafe { FLAG.flimit } > 0 && n > unsafe { FLAG.flimit } as i64 {
                            err = Some(format!(
                                "{} entries exceeds filelimit, not opening dir",
                                n
                            ));
                            unsafe { ERRORS += 1; }
                            subdir = None;
                        }
                    }
                    // C: if (subdir == NULL) descend = 0;
                    if subdir.is_none() {
                        descend = 0;
                    }
                }
            }
        } else {
            // C: else tot.files++;
            tot.files += 1;
        }

        // C: needsclosed = lc.printfile(dirname, filename, *dir, descend + htmldescend + (flag.J && errors));
        let needsclosed = (lc().printfile)(
            dirname,
            &filename,
            Some(&dir[idx]),
            descend + htmldescend + if unsafe { FLAG.J } && unsafe { ERRORS } != 0 { 1 } else { 0 },
        );
        // C: if (err) lc.error(err);
        if let Some(e) = &err {
            (lc().error)(e);
        }

        // C: if (descend > 0)
        if descend > 0 {
            // C: lc.newline(*dir, lev, 0, 0);
            (lc().newline)(Some(&dir[idx]), lev, 0, false);
            // C: subtotal = listdir(newpath, subdir, lev+1, dev, hasfulltree);
            let sub = subdir.take().expect("descend>0 时 subdir 非空（C 中该条件保证）");
            let subtotal = listdir(&newpath, sub, lev + 1, dev, hasfulltree);
            tot.dirs += subtotal.dirs;
            tot.files += subtotal.files;
        } else if needsclosed == 0 {
            // C: else if (!needsclosed) lc.newline(*dir, lev, 0, *(dir+1)!=NULL);
            (lc().newline)(Some(&dir[idx]), lev, 0, idx + 1 < dir.len());
        }

        // C: if (subdir) { free_dir(subdir); subdir = NULL; } —— Rust 中已消费或为 None

        // C: if (needsclosed) lc.close(*dir, descend? lev : -1, *(dir+1)!=NULL);
        if needsclosed != 0 {
            (lc().close)(
                Some(&dir[idx]),
                if descend != 0 { lev } else { -1 },
                idx + 1 < dir.len(),
            );
        }

        // C: if (*(dir+1) && !*(dir+2)) dirs[lev] = 2;
        if idx + 1 < dir.len() && idx + 2 >= dir.len() {
            unsafe { DIRS[lev as usize] = 2 };
        }

        // C: if (ig != NULL) ig = pop_filterstack();
        if ig.is_some() {
            pop_filterstack();
            ig = None;
        }
        // C: if (inf != NULL) inf = pop_infostack();
        if inf.is_some() {
            pop_infostack();
            inf = None;
        }
        idx += 1;
    }

    // C: dirs[lev] = 0;
    unsafe { DIRS[lev as usize] = 0 };
    tot
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    // 捕获输出到内存，供测试断言
    #[derive(Clone)]
    struct Capture {
        buf: Arc<Mutex<Vec<u8>>>,
    }
    impl Write for Capture {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            self.buf.lock().unwrap_or_else(|e| e.into_inner()).extend_from_slice(data);
            Ok(data.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    // report 回调记录统计结果
    static REPORTED: Mutex<Option<Totals>> = Mutex::new(None);
    fn test_report(t: Totals) {
        *REPORTED.lock().unwrap_or_else(|e| e.into_inner()) = Some(t);
    }
    // 空操作回调（unix 输出实现在 unix.rs，尚未翻译；此处仅验证遍历逻辑）
    fn test_printinfo(_d: &str, _f: Option<&mut Info>, _l: i32) -> i32 {
        0
    }
    fn test_printfile(_d: &str, _f: &str, _i: Option<&Info>, _desc: i32) -> i32 {
        0
    }
    fn test_error(_e: &str) -> i32 {
        0
    }
    fn test_newline(_f: Option<&Info>, _l: i32, _p: i32, _c: bool) {}
    // close 回调的测试替身（当前测试未直接引用，保留以对应 ListingCalls 完整签名）
    #[allow(dead_code)]
    fn test_close(_f: Option<&Info>, _l: i32, _c: bool) {}

    // 初始化全局状态（FLAG 默认、DIRS、LC、输出流）
    fn setup() -> Capture {
        let cap = Capture {
            buf: Arc::new(Mutex::new(Vec::new())),
        };
        // unsafe：测试中初始化全局状态
        unsafe {
            FLAG = crate::tree::Flags::new();
            DIRS.resize(crate::tree::PATH_MAX, 0);
            LC = Some(ListingCalls {
                intro: null_intro,
                outtro: null_outtro,
                printinfo: test_printinfo,
                printfile: test_printfile,
                error: test_error,
                newline: test_newline,
                close: null_close,
                report: test_report,
            });
            OUTFILE = Some(Box::new(cap.clone()));
            crate::globals::LEVEL = -1;
            crate::globals::ERRORS = 0;
            crate::globals::TOPSORT = None;
            // GETFULLTREE 在测试中保持 None（测试均走 needfulltree=false 路径）
        }
        cap
    }

    #[test]
    fn test_emit_tree_counts() {
        // 共享 FLAG/OUTFILE/DIRS/LC，串行化
        let _lock = crate::globals::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _cap = setup();
        // 构造临时目录树：tmp/a.txt、tmp/sub/b.txt
        let tmp = std::env::temp_dir().join(format!(
            "rustree_list_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let sub = tmp.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(tmp.join("a.txt"), "a").unwrap();
        std::fs::write(sub.join("b.txt"), "b").unwrap();

        let mut dirs = vec![tmp.to_string_lossy().into_owned()];
        emit_tree(&mut dirs, false);

        let reported = REPORTED.lock().unwrap_or_else(|e| e.into_inner()).take().expect("report 被调用");
        // 文件：a.txt + b.txt = 2；目录：sub（listdir 内）+ 顶层 tmp（emit_tree 的 subtotal.dirs++）= 2
        assert_eq!(reported.files, 2);
        assert_eq!(reported.dirs, 2);

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn test_emit_tree_empty_dir() {
        // 共享 FLAG/OUTFILE/DIRS/LC，串行化
        let _lock = crate::globals::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _cap = setup();
        let tmp = std::env::temp_dir().join(format!(
            "rustree_list_empty_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).unwrap();

        let mut dirs = vec![tmp.to_string_lossy().into_owned()];
        emit_tree(&mut dirs, false);

        let reported = REPORTED.lock().unwrap_or_else(|e| e.into_inner()).take().expect("report 被调用");
        assert_eq!(reported.files, 0);
        assert_eq!(reported.dirs, 0);

        std::fs::remove_dir_all(&tmp).ok();
    }
}



