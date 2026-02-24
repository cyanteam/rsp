# RSP - Rust Server Pages: 让rust实现php的“动态页面”


用 Rust 写的类 PHP 模板引擎，写法跟 JSP/PHP 差不多，但用的是 Rust。

## 为什么要这个？

写 Web 用 Rust，生态没 PHP 那么方便。这个东西就是让你能像写 PHP 那样写页面，但背后是 Rust 的性能和类型安全。（~~人话：闲得发慌搓的~~）

第一次访问会编译，之后直接加载缓存的动态库，速度很快。

## 安装

### 下载release
打开项目的release页面，下载对应的版本即可。

### 手动编译

```bash
cargo build --release
```

编译产物在 `target/release/rsp`

## 怎么用

### 基本例子

```rsp
<!DOCTYPE html>
<html>
<body>
    <h1>Hello, RSP!</h1>
    
    <% let name = "World"; %>
    <p>Hello, <%= name %>!</p>
    
    <% for i in 0..5 { %>
        <li>Item <%= i + 1 %></li>
    <% } %>
</body>
</html>
```

运行：
```bash
./target/release/rsp examples/hello.rsp
```

### 模板标签

| 标签 | 干啥的 |
|------|--------|
| `<% code %>` | 执行 Rust 代码 |
| `<%= expr %>` | 输出内容到页面（就像php的<?=$a?>） |
| `<%! code %>` | 静态声明，整个进程只执行一次 |
| `<%@ use xxx %>` | 导入 Rust 模块 |
| `<%@ dep xxx %>` | 加依赖，类似 Cargo.toml（其实本质上就是） |
| `<%@ once_cell %>` | 启用懒加载 static（比如说数据库只连一次） |

### 获取请求参数

```rsp
<%
    // GET 参数
    let name = req.get.or("name", "默认值");
    let id = req.get.str("id");        // 返回 &str
    
    // POST 参数
    let content = req.post.str("content");
    
    // Cookie
    let session = req.cookie["PHPSESSID"];
    
    // 请求头
    let ua = req.ua["user-agent"];
%>
```

### 响应控制

```rsp
<%
    // 设置状态码
    header(404);
    header(500);
    
    // 跳转
    header_url("/login");
    
    // 设置 Cookie，单位秒
    SetCookie("token", "abc123", 3600);
    
    // 删除 Cookie
    CleanCookie("token");
%>
```

### SQL连接

```rsp
<%@ once_cell %>
<%@ dep rusqlite = { version = "0.32", features = ["bundled"] } %>
<%@ use rusqlite::Connection %>
<%@ use std::sync::Mutex %>
<%!
static DB: Lazy<Mutex<Connection>> = Lazy::new(|| {
    let conn = Connection::open("forum.db").unwrap();
    Mutex::new(conn)
});
%>

<%
    let conn = DB.lock().unwrap();
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM posts", [], |r| r.get(0)
    ).unwrap_or(0);
%>

<p>帖子数: <%= count %></p>
```

## 启动服务

```bash
# 简单启动
./target/release/rsp -S 0.0.0.0:8080 -t ./

^^^^^^^^^^^^^软件位置 ^启动服务 ^IP:端口 ^执行目录

# 指定首页
./target/release/rsp -S 0.0.0.0:8080 -t ./ -i index.rsp
^^^^^^^^^^^^^软件位置 ^启动服务 ^IP:端口 ^执行目录 ^指定首页文件

```

访问 `http://localhost:8080` 就会跑 `index.rsp`。

## 缓存说明

- 编译出来的 `链接库` 存在 `.rspcache/` 目录下
- Cargo 依赖缓存在 `~/.rsp/target/`（不会自动清理）
- 改完 rsp 文件自动重新编译

## 目录结构

```
src/
├── main.rs        # 入口
├── engine.rs      # 核心
├── compiler.rs   # 编译
├── generator.rs  # 代码生成
├── loader.rs     # 动态库加载
└── parser.rs     # 解析

runtime/src/
├── request.rs    # 请求相关
├── db.rs         # 数据库
└── response.rs  # 响应

examples/
├── hello.rsp
├── demo.rsp
└── forum/        # 论坛 demo
```

## TODO

- [ ] include 指令（嵌入其他 rsp）
- [ ] 热更新
- [ ] 更多数据库支持
- [ ] 指定页面（如php的laravel框架，指定执行public/index.php）
- [ ] 修改examples

---

就这样，有问题看 examples 目录（还没写还没写）。