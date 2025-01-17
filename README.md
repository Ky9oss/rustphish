<div align="center">
    <h1>Rustphish</h1>
    <img src="https://img.shields.io/badge/Made%20with-Rust-1f425f.svg" alt="made-with-rust">
    <img src="https://img.shields.io/badge/Maintained%3F-yes-green.svg" alt="Maintenance">
    <img src="https://img.shields.io/badge/Maintainer-Ky9oss-red" alt="Maintainer">
    <img src="https://img.shields.io/badge/rustc-1.84.0(nightly)-blue.svg" alt="rustc-version">
    <img src="https://img.shields.io/badge/compile-gnu-blue.svg" alt="Compilation-Chain">
    <br>
    <br>
    <img src="img/1.png" alt="" width="213.5" height="203.5">
</div>

---

[简体中文](./README.md) | [English](./README_EN.md)

一个client-server分离的轻量化、高效能的进阶邮件钓鱼工具，旨在替代`gophish`

# 快速开始
```bash
./bin/client.exe --help
```
![](img/2.png)

# 项目介绍
![](img/4.png)

# 使用方法
## 搭建服务器
1. 修改`config.toml`，配置服务端监听端口、数据库路径、smtp服务器信息等，详情见[配置文件](#配置文件)
2. 修改`frontend/index.html`，配置仿冒页面，详情见[仿冒页面](#仿冒页面)
3. 运行`./server`，启动服务端
4. 所有信息自动记录进`./database`数据库，该数据库只能使用客户端软件读取：`./client.exe --read ./database`

## 使用客户端
1. `./client.exe --input xxx.txt`: 导入邮箱
2. `./client.exe --show`: 确认邮箱成功导入
3. 修改配置文件`config.toml`，配置每封邮件的间隔时间、发信人、主题等信息，详情见[配置文件](#配置文件)
4. 修改邮件模板文件`template.html`，配置邮件内容，详情见[邮件模板](#邮件模板)
5. `./client.exe --send-all`: 发送所有钓鱼邮件

## 注意事项
### 仿冒页面
> 建议使用`form`和`input`标签完成提交功能，确保`post`包提交到服务器的`{{submit}}`接口
请下方为最简化的仿冒页面，根据需要自行修改
```html
<!DOCTYPE html>
<html>
<head>
    <title>Dynamic Form</title>
</head>
<body>
    <!-- 重要部分 -->
    <form action="{{submit}}" method="post">
        <input type="text" name="key1"><br>
        <input type="text" name="key2"><br>
        <input type="text" name="key3"><br>
        <button type="submit">Submit</button>
    </form>
</body>
</html>
```

### 邮件模板
下方提供一个最小化的钓鱼模板，其中`index`和`image`是服务端提供的接口，`{{id}}`在程序运行时会被自动替换为受害人的id
所以你**只需要**修改ip和port，请不要修改url路径接口
```html
<html><head>
点击下方链接，完成测试： <a href="http://ip:port/index/{{id}}">http://ip:port/index/{{id}}</a>
<img src="http://ip:port/image/{{id}}" alt=""> <!-- 用于记录受害者是否打开邮件 -->
</body></html>
```

### 配置文件
- `server.ip`：0.0.0.0
- `server.port`：服务端端口
- `paths.phish_page`：仿冒页面路径
- `paths.redirect_url`：提交成功后，重定向的url
- `paths.success_page`：路由`/success`下的成功页面路径，可以用于`paths.redirect_url`重定向
- `smtp.server`：smtp服务器地址
- `smtp.username`：smtp服务器用户名
- `smtp.from_email`：发件人邮箱
- `smtp.subject`：邮件主题
- `smtp.interval`：每封邮件间隔时间
- `email.template`：邮件模板路径

# 编译
```bash
cargo build --release -p client
cargo build --release -p server
```

# 功能实现
## 服务端
- [x] 图片识别接口
- [x] 外部config
- [x] 替换unwrap，确保稳定性
- [ ] https
- [ ] 挂载木马

## 客户端
- [x] 导入受害人列表，存储受害人信息及对应ID
- [x] 利用邮件模板和受害人信息，生成对应钓鱼邮件
- [x] 读取服务端数据库，格式化输出所有钓鱼信息
- [x] 使用smtp通过其他邮箱平台发送邮件

