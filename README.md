<img src="https://github.com/user-attachments/assets/bc7fe8b8-ca25-4d9d-8de3-24a717068d9a" alt="zitefy" width="250" align="right">

# `server` 🖥️

> This is the backend server of zitefy, which manages all the user data, templates, website previews and basically everything. Written in rust, it uses the actix web framework with some other crates.

## Table Of Contents
* [What's this?](#whats-this)
   * [Role in the stack](#role-in-the-stack)
   * [Architectural Overview](#architectural-overview)
* [Routes](#routes)
* [Security](#security)
* [Setup Guide](#setup-guide)
   * [Prerequisites](#prerequisites)
   * [Installation](#installation)
   * [Build & run](#build--run)
* [Contributing](#contributing)

## What's this?
In order for the core functionalities to work, there should be a central system where templates, users & their sites along with other stuff like assets, seo data can come together. This server is the means for doing so.

### Role in the stack
* Maintain and store user data, site data & template data
* Provide a storage mechanism for storing site assets, seo data, etc
* Generate previews for templates, sites & the web based editor

### Architectural Overview
This is written from the high level system architecture shown below. You can find some pointers below highlighting the core idea in a nutshell. Feel free to open a new issue if you find any part of this documentation confusing or hard to understand.

<img width="1005" alt="architecture" src="https://github.com/user-attachments/assets/656b22af-9c7a-46d8-899c-74bf30a32c64">

*In a nutshell...*
  * Each user can register themselves on the API. Once registered or logged in, the API will generate an access token that can then be used to access the secure endpoints. Registered users can create sites from the available templates.
  * In zitefy, only user data is kept private. This is an open platform and so, all source code & data related to all sites & templates will be exposed publicly. This is a reminder to not upload any data to a site that you don't want the world to see.
  * Templates are uploaded and managed from the [templates]() repository. The CI/CD uploads the templates to a dedicated directory on the server and a background task that runs once every hour will update the template data in the database.
  * The preview engine is basically two JS scripts. One is the [`scripts/builder.js`]() that builds an HTML string from the given data. Another is the [`scripts/screenshot.js`]() that takes a screenshot of the generated HTML.

The intention was to keep this a pure rust codebase, but it turns out that there are no effective html parsers in rust. Neither are there any methods to take a screenshot of a webpage. So, we ended up writing two JS files for those and only those. The rest is pure rust.

The server then binds with two ports. Then, [nginx](https://nginx.org/en/0) redirects traffic from both [api.zitefy.com](https://api.zitefy.com/docs/#/) & [zitefy.com](https://zitefy.com/) to the respective ports. There is a [systemd](https://systemd.io/) service that manages everything.

It would be cool to containerize this server as it'd speed up CI/CD for future versions. But right now, the setup overhead seemed a bit too much for the hackathon so we skipped it.

## Routes
Routes are of 4 types: `site`, `template`, `user` & `proxy`. Yeah, unfortunately proxy is a category. Anthropic, regardless of maintaining a typescript SDK, [doesn't allow CORS](https://github.com/anthropics/anthropic-sdk-typescript/issues/410) in their API rendering it unusable for web apps. If this had worked, it would have reduced our code complexity as well, but since it doesn't, proxy is a thing.

As it should be obvious, listing all the routes here seems tedious. So, they're documented in this [swagger UI](https://api.zitefy.com/docs/#/).

## Security
The focus up until now has been somehow getting this to work properly for the hackathon. Security wasn't the first thing in our mind when building this. As such, it is advised not to upload any sensitive info to zitefy.

Here are the measures taken for security:
    * All passwords are encrypted using a hashing algorithm with a secret key. Without it, the API will not allow access to user data
    * All logins are regulated by access tokens which expire every 30 days. After 30 days, you'd have to login again to continue using the site
    * The secret key used for hashing, along with some other stuff, are stored in a separate file not exposed in this repository.

If you find a security vulnerability, please do open an issue/PR and we'd be happy to accept it!

## Setup Guide
You can follow this guide to run the server on a machine of your choice. If you want to deploy, it's advisable to do so in a linux system. There is nothing OS specific about the code itself, and so it can build and run on any platform. However, it'd be better to deploy on a linux machine since the secrets and service file are setup to be managed by systemd. When you open a PR, it'll be much easier for us to evaluate. If you're on Windows, WSL would be more than enough.

### Prerequisites
* The **Rust toolchain**, including cargo.

    Install for linux/WSL by running this command
    ```
     $ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```
    Alternatively, follow the instruction method for your platform [here](https://www.rust-lang.org/tools/install)

* **Git CLI**

    For linux, skip this step. Install by running the executable for your platform available [here](https://git-scm.com/downloads)

* **MongoDB Community Server**

    Install for Ubuntu by following [this guide](https://www.mongodb.com/docs/manual/tutorial/install-mongodb-on-ubuntu/). For other operating systems, follow the corresponding tutorial from the [tutorials](https://www.mongodb.com/docs/manual/installation/#mongodb-installation-tutorials) page. If you want to visualize the data better, install MongoDB Compass by following [this guide](https://www.mongodb.com/docs/compass/current/install/).

* **bun**

    Install for linux/WSL or macOS by running this command
    ```
     $ curl -fsSL https://bun.sh/install | bash
    ```
    Alternatively, you can follow the instructions for your platform [here](https://bun.sh/).

### Installation
1. Clone the repository to your machine.

    ```
    git clone https://github.com/zitefy/server.git
    ```

2. Install JavaScript dependencies

    ```
    bun install puppeteer
    ```
    ```
    bun run puppeteer browsers install chrome
    ```
3. Copy the chrome executable path to line 8 in [`scripts/screenshot.js`]().

### Build & run
1. Create an env file with all your secrets. An example file would look like this
    
    ```env
    MONGODB_URI="mongodb://127.0.0.1:27017/"
    API_ADDR=127.0.0.1:7878
    SERVER_ADDR=127.0.0.1:7979
    SECRET_KEY="xxxx"
    ANTHROPIC_KEY='xxxx'
    ```
    You can get your anthropic API key [here](https://console.anthropic.com/settings/keys)
2. Build and run the server

    ```
    cargo run
    ```

That's about it, the server should be up and running locally in your machine.

## Contributing
There's tons of stuff to contribute. Please refer to the [contributing guide](https://github.com/zitefy/server/blob/main/CONTRIBUTING.md).
