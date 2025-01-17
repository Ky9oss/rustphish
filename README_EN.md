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

English | [简体中文](./README.md)

A lightweight, high-performance advanced phishing tool with client-server separation, designed to replace `gophish`

# Quick Start
```bash
./bin/client.exe --help
```
![](img/2.png)

# Project Introduction
![](img/4.png)

# Usage
## Server Setup
1. Modify `config.toml` to configure server listening port, database path, SMTP server information, etc., see [Configuration File](#configuration-file) for details
2. Modify `frontend/index.html` to configure the phishing page, see [Phishing Page](#phishing-page) for details
3. Run `./server` to start the server
4. All information is automatically recorded in the `./database`, which can only be read using the client software: `./client.exe --read ./database`

## Client Usage
1. `./client.exe --input xxx.txt`: Import email addresses
2. `./client.exe --show`: Confirm successful email import
3. Modify `config.toml` to configure email interval, sender, subject, etc., see [Configuration File](#configuration-file) for details
4. Modify `template.html` to configure email content, see [Email Template](#email-template) for details
5. `./client.exe --send-all`: Send all phishing emails

## Notes
### Phishing Page
> It's recommended to use `form` and `input` tags for submission, ensuring the `post` package is submitted to the server's `{{submit}}` endpoint
Below is a minimized phishing page template, modify as needed:
```html
<!DOCTYPE html>
<html>
<head>
    <title>Dynamic Form</title>
</head>
<body>
    <!-- Important part -->
    <form action="{{submit}}" method="post">
        <input type="text" name="key1" value="value1"><br>
        <input type="text" name="key2" value="value2"><br>
        <input type="text" name="key3" value="value3"><br>
        <button type="submit">Submit</button>
    </form>
</body>
</html>
```

### Email Template
Below is a minimized phishing template. The `index` and `image` are server-provided endpoints, and `{{id}}` will be automatically replaced with the victim's ID at runtime.
You **only need to** modify the ip and port, please don't modify the URL path endpoints:
```html
<html><head>
Click the link below to complete the test: <a href="http://ip:port/index/{{id}}">http://ip:port/index/{{id}}</a>
<img src="http://ip:port/image/{{id}}" alt=""> <!-- Used to track if victim opens the email -->
</body></html>
```

# Compilation
```bash
cargo build --release -p client
cargo build --release -p server
```

# Features
## Server
- [x] Image tracking endpoint
- [x] External config
- [x] Replace unwrap for stability
- [ ] HTTPS support
- [ ] Malware deployment

## Client
- [x] Import victim list, store victim info and corresponding IDs
- [x] Generate phishing emails using templates and victim info
- [x] Read server database, format and display all phishing information
- [x] Send emails via SMTP through other email platforms 

### Configuration File
- `server.ip`: 0.0.0.0
- `server.port`: Server listening port
- `paths.phish_page`: Path to phishing page
- `paths.redirect_url`: URL to redirect after successful submission
- `paths.success_page`: Path to success page under `/success` route, can be used as `paths.redirect_url`
- `smtp.server`: SMTP server address
- `smtp.username`: SMTP server username
- `smtp.from_email`: Sender's email address
- `smtp.subject`: Email subject
- `smtp.interval`: Interval between each email
- `email.template`: Path to email template 