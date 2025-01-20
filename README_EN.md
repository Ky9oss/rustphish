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

A lightweight, high-performance phishing email drill tool with client-server separation, designed to replace `gophish`

# Disclaimer
This tool is only intended for **legally authorized** enterprise security construction activities. If you need to test the availability of this tool, please set up your own target machine environment.

When using this tool for testing, you should ensure that the behavior complies with local laws and regulations and that sufficient authorization has been obtained. **Do not conduct phishing against unauthorized targets.**

If you engage in any illegal behavior while using this tool, you will need to bear the corresponding consequences yourself, and we will not bear any legal or joint liability.

Before installing and using this tool, please **carefully read and fully understand the content of each clause**. Limitations, disclaimers, or other clauses involving your significant rights may be highlighted in bold, underlined, or other forms to draw your attention. Unless you have fully read, completely understood, and accepted all terms of this agreement, please do not install and use this tool. Your use of the tool or your acceptance of this agreement in any other express or implied manner shall be deemed that you have read and agreed to be bound by this agreement.

# Quick Start
```bash
./bin/client.exe --help
```
![](img/2.png)

# Project Introduction
![](img/4.png)

# Usage
## Download
Download `client` and `server` from the `Releases` section on the right side of Github according to your operating system environment
> `client_read` is a minimally compiled client that only reads databases and doesn't support sending emails. It's designed for scenarios where database files in internal networks are inconvenient to transfer out. This file can be transferred to internal servers for direct database record decryption
> `client_all` is the complete client that supports sending emails

## Server Setup
1. Modify `config.toml` to configure server listening port, database path, SMTP server information, etc., see [Configuration File](#configuration-file) for details
2. Modify `frontend/index.html` to configure the phishing page, see [Phishing Page](#phishing-page) for details
3. Run `./server` to start the server
4. All information is automatically recorded in the `./database`, which can only be read using the client software: `./client.exe --read ./database`

## Using Client to Send Phishing Emails
1. `./client.exe --input xxx.txt`: Import email addresses
2. `./client.exe --show`: Confirm successful email import
3. Modify `config.toml` to configure email interval, sender, subject, etc., see [Configuration File](#configuration-file) for details
4. Modify `template.html` to configure email content, see [Email Template](#email-template) for details
5. `./client.exe --send-all`: Send all phishing emails

## Using Client to Read Database Records
1. Ensure the `email_database` file contains the corresponding victim email information and IDs, confirm with `client.exe --show`
2. `./client.exe --read ./database`: Read database records (or use `client_read`)

## Notes
### Important Files
- `email_database`: Contains victim email information and IDs, recorded when initially importing emails with `--input`. After sending phishing emails, do not delete this file or use `--delete` to remove emails, otherwise database records cannot be read. This file can be copied, backed up, and replaced in other clients' same-named files for reading database records
- `database`: Server database, do not delete this file after sending phishing emails, otherwise victim access records cannot be read. Recommended to backup

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
        <input type="text" name="key1"><br>
        <input type="text" name="key2"><br>
        <input type="text" name="key3"><br>
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

### Configuration File
```toml
[server]
ip = "0.0.0.0" #Server listening IP
port = 8080 #Server port

[paths]
phish_page = "./frontend/test.html" #Path to phishing page
redirect_url = "http://localhost:8080/success" #URL to redirect after successful submission
success_page = "./frontend/success.html" #Path to success page under `/success` route, can be used as `paths.redirect_url`

[smtp]
server = "smtp.126.com" #SMTP server address
username = "xxx@126.com" #SMTP server username
from_email = "Test <xxx@126.com>" #Sender's email address
subject = "Test Subject" #Email subject
interval = 5 #Interval between each email

[email]
template = "template.html" #Path to email template
```

# Compilation
```bash
cargo build --release -p client --features db
cargo build --release -p client --all-features
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