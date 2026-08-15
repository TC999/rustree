// 文件路径：src/hash.rs
// 对应 C 源文件：hash.c
// uid/gid -> 名称的快速哈希缓存表、inode 记录表（防止重复跟随符号链接）、
// 以及 Linux 专属的字符串驻留表（strhash）。

/* ---------------------------------------------------------------------
 * 哈希桶数：256（C: #define HASH(x) ((x)&255) / #define inohash(x) ((x)&255)）
 * 取 key 的低 8 位作为桶索引。
 * --------------------------------------------------------------------- */

// C: #define HASH(x) ((x)&255)
fn hash_index(x: u32) -> usize {
    (x & 255) as usize
}

// C: #define inohash(x) ((x)&255)
fn inohash_index(x: u64) -> usize {
    (x & 255) as usize
}

// C: struct xtable { unsigned int xid; char *name; struct xtable *nxt; }
// 用户/组名缓存节点，桶内按 xid 升序排列
struct Xtable {
    xid: u32,
    name: String,
    nxt: Option<Box<Xtable>>,
}

// C: struct inotable { ino_t inode; dev_t device; struct inotable *nxt; }
// inode 记录节点，桶内按 (inode, device) 升序排列
struct Inotable {
    inode: u64,
    device: u64,
    nxt: Option<Box<Inotable>>,
}

// C: struct xtable *gtable[256], *utable[256];
// 组名表与用户名表。
// 注意：[None; 256] 要求元素类型为 Copy，而 Option<Box<T>> 不是 Copy，
// 因此用 inline const 表达式 [const { None }; 256] 逐项构造。
static mut GTABLE: [Option<Box<Xtable>>; 256] = [const { None }; 256];
static mut UTABLE: [Option<Box<Xtable>>; 256] = [const { None }; 256];

// C: struct inotable *itable[256];
// inode 记录表
static mut ITABLE: [Option<Box<Inotable>>; 256] = [const { None }; 256];

// C: struct strtable { char *string; struct strtable *nxt; } *strtable[256];（仅 Linux）
// 字符串驻留表，桶内按字符串字节序排列
#[cfg(target_os = "linux")]
struct Strtable {
    string: String,
    nxt: Option<Box<Strtable>>,
}

#[cfg(target_os = "linux")]
static mut STRTABLE: [Option<Box<Strtable>>; 256] = [const { None }; 256];

// === 原 C 函数：char *strhash(char *str) ===（仅 Linux）
/// 将字符串按 DJB2 哈希驻留到全局表中，重复字符串返回同一个值。
/// C 返回驻留指针，Rust 返回克隆的 String（值等价，且表项在程序结束前不被释放）。
#[cfg(target_os = "linux")]
pub fn strhash(str_: &str) -> String {
    // DJB2 哈希。C 中 hash 为 unsigned int，算术溢出为环绕语义，
    // 因此 Rust 使用 wrapping 运算保持行为一致（debug 模式下普通 + 会 panic）。
    let mut hash: u32 = 5381;
    for &b in str_.as_bytes() {
        hash = hash.wrapping_shl(5).wrapping_add(hash).wrapping_add(b as u32);
    }
    let hp = (hash & 255) as usize;

    // unsafe：访问全局驻留表 STRTABLE 并进行有序链表查找/插入（对应 C 的 strtable）
    unsafe {
        // C 中通过指针遍历链表；Rust 用裸指针 cur 模拟（对应 C 的 s/p/n 指针），
        // 避免借用检查器对可变链表操作的限制
        let mut cur: *mut Option<Box<Strtable>> = &mut STRTABLE[hp];
        loop {
            match (*cur).as_mut() {
                None => break,
                Some(node) => {
                    // C: c = strcmp(s->string, str); if (c == 0) return s->string;
                    let cmp = node.string.as_str().cmp(str_);
                    if cmp == std::cmp::Ordering::Equal {
                        return node.string.clone();
                    }
                    // C: if (c > 0) break;
                    if cmp == std::cmp::Ordering::Greater {
                        break;
                    }
                    cur = &mut node.nxt;
                }
            }
        }

        // 插入新节点（对应 C 的 xmalloc + scopy + 有序链表插入）
        let n = Box::new(Strtable {
            string: str_.to_string(),
            nxt: (*cur).take(),
        });
        *cur = Some(n);
        str_.to_string()
    }
}

// === 原 C 函数：void init_hashes(void) ===
/// 初始化哈希表。
/// C 中 memset 将 utable/gtable/itable/strtable 清零；
/// Rust 的 static 数组初始即为 None（空桶），无需显式清零。
/// 保留此函数以对应 C 中的调用点（main 中调用）。
pub fn init_hashes() {}

// === 原 C 函数：char *uidtoname(uid_t uid) ===
/// 将 uid 解析为用户名；未命中时通过 getpwuid 查询并缓存到哈希表。
pub fn uidtoname(uid: u32) -> String {
    let uent = hash_index(uid);

    // unsafe：访问全局用户名表 UTABLE 并进行有序链表查找/插入（对应 C 的 utable）
    unsafe {
        // 查找插入点：链表按 xid 升序，cur 停在第一个 xid >= uid 的节点
        // C: for(o = p = utable[uent]; p ; p=p->nxt) {
        //      if (uid == p->xid) return p->name;
        //      else if (uid < p->xid) break;
        //      o = p;
        //    }
        let mut cur: *mut Option<Box<Xtable>> = &mut UTABLE[uent];
        loop {
            match (*cur).as_mut() {
                None => break,
                Some(node) => {
                    if node.xid == uid {
                        return node.name.clone();
                    }
                    if node.xid > uid {
                        break;
                    }
                    cur = &mut node.nxt;
                }
            }
        }

        // 未找到：进行真实查询并加入表
        // C: if ((ent = getpwuid(uid)) != NULL) t->name = scopy(ent->pw_name);
        //     else { snprintf(ubuf,30,"%d",uid); ubuf[31]=0; t->name = scopy(ubuf); }
        let name = crate::sys::uid_name(uid).unwrap_or_else(|| uid.to_string());
        let ret = name.clone();
        let t = Box::new(Xtable {
            xid: uid,
            name,
            nxt: (*cur).take(),
        });
        *cur = Some(t);
        ret
    }
}

// === 原 C 函数：char *gidtoname(gid_t gid) ===
/// 将 gid 解析为组名；未命中时通过 getgrgid 查询并缓存到哈希表。
pub fn gidtoname(gid: u32) -> String {
    let gent = hash_index(gid);

    // unsafe：访问全局组名表 GTABLE 并进行有序链表查找/插入（对应 C 的 gtable）
    unsafe {
        let mut cur: *mut Option<Box<Xtable>> = &mut GTABLE[gent];
        loop {
            match (*cur).as_mut() {
                None => break,
                Some(node) => {
                    if node.xid == gid {
                        return node.name.clone();
                    }
                    if node.xid > gid {
                        break;
                    }
                    cur = &mut node.nxt;
                }
            }
        }

        // C: if ((ent = getgrgid(gid)) != NULL) t->name = scopy(ent->gr_name);
        //     else { snprintf(gbuf,30,"%d",gid); gbuf[31]=0; t->name = scopy(gbuf); }
        let name = crate::sys::gid_name(gid).unwrap_or_else(|| gid.to_string());
        let ret = name.clone();
        let t = Box::new(Xtable {
            xid: gid,
            name,
            nxt: (*cur).take(),
        });
        *cur = Some(t);
        ret
    }
}

// === 原 C 函数：void saveino(ino_t inode, dev_t device) ===
/// 记录已跟随的符号链接的 (inode, device)，避免重复跟随造成递归。
pub fn saveino(inode: u64, device: u64) {
    let hp = inohash_index(inode);

    // unsafe：访问全局 inode 表 ITABLE 并进行有序链表查找/插入（对应 C 的 itable）
    unsafe {
        // 查找插入点：链表按 (inode, device) 升序
        // C: for(pp = ip = itable[hp]; ip; ip = ip->nxt) {
        //      if (ip->inode > inode) break;
        //      if (ip->inode == inode && ip->device >= device) break;
        //      pp = ip;
        //    }
        let mut cur: *mut Option<Box<Inotable>> = &mut ITABLE[hp];
        loop {
            match (*cur).as_mut() {
                None => break,
                Some(node) => {
                    if node.inode > inode {
                        break;
                    }
                    if node.inode == inode && node.device >= device {
                        break;
                    }
                    cur = &mut node.nxt;
                }
            }
        }

        // C: if (ip && ip->inode == inode && ip->device == device) return;
        if let Some(node) = (*cur).as_deref() {
            if node.inode == inode && node.device == device {
                return;
            }
        }

        // 插入新记录
        let it = Box::new(Inotable {
            inode,
            device,
            nxt: (*cur).take(),
        });
        *cur = Some(it);
    }
}

// === 原 C 函数：bool findino(ino_t inode, dev_t device) ===
/// 查询 (inode, device) 是否已记录。
pub fn findino(inode: u64, device: u64) -> bool {
    let hp = inohash_index(inode);

    // unsafe：访问全局 inode 表 ITABLE（对应 C 的 itable）
    unsafe {
        let mut node = ITABLE[hp].as_deref();
        while let Some(n) = node {
            // C: if (it->inode > inode) break;
            if n.inode > inode {
                break;
            }
            // C: if (it->inode == inode && it->device >= device) break;
            if n.inode == inode && n.device >= device {
                break;
            }
            node = n.nxt.as_deref();
        }
        // C: if (it && it->inode == inode && it->device == device) return true;
        if let Some(n) = node {
            if n.inode == inode && n.device == device {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    // 哈希表（UTABLE/GTABLE/ITABLE/STRTABLE）为全局 static mut，
    // cargo test 多线程并行时须串行化访问（与 filter.rs 的 STACK_LOCK 同理）
    fn with_lock<T>(f: impl FnOnce() -> T) -> T {
        let _g = crate::globals::TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        f()
    }

    #[test]
    fn test_saveino_findino() {
        with_lock(|| {
            // 初始未记录
            assert!(!findino(100, 200));
            saveino(100, 200);
            assert!(findino(100, 200));
            // 相同 inode 不同 device 不算命中
            assert!(!findino(100, 201));
            // 再次保存同记录不产生重复
            saveino(100, 200);
            assert!(findino(100, 200));
            // 不同 inode
            assert!(!findino(101, 200));
            // 哈希碰撞（不同桶）互不影响
            saveino(300, 1);
            assert!(findino(300, 1));
            assert!(!findino(44, 1));
        });
    }

    #[test]
    fn test_uidtoname_consistent() {
        with_lock(|| {
            // 同一 uid 重复查询应返回相同名称（验证缓存命中路径）
            let a = uidtoname(12345);
            let b = uidtoname(12345);
            assert_eq!(a, b);
            // 未命中的 uid 在无 getpwuid 的平台回退为数字字符串
            assert_eq!(uidtoname(12345), b);
        });
    }

    #[test]
    fn test_gidtoname_consistent() {
        with_lock(|| {
            let a = gidtoname(23456);
            let b = gidtoname(23456);
            assert_eq!(a, b);
        });
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_strhash() {
        with_lock(|| {
            // 驻留：同一字符串返回相同值
            assert_eq!(strhash("abc"), strhash("abc"));
            assert_eq!(strhash(""), strhash(""));
            // 不同字符串
            let a = strhash("aaa");
            let b = strhash("aab");
            assert_ne!(a, b);
        });
    }
}

