<!--
SPDX-FileCopyrightText: The redclock Authors
SPDX-License-Identifier: 0BSD
 -->

# redclock

`redclock` is a CLI to track time entries in [Redmine](https://www.redmine.org/).

## Installation

Use `cargo` to compile & install `redclock`:

```shell
$ cargo install redclock
```

Or download an appropriate binary from our [release page](https://github.com/metio/redclock/releases/latest).

## Usage

### Servers

`redclock` saves time entries in a Redmine instance, therefore you need to add a server to your local configuration first:

```shell
$ redclock server add https://redmine.example.com --token <YOUR_API_TOKEN_HERE>
```

In case you are using a password manager that offers a CLI, use this instead for maximum security:

```shell
$ redclock server add https://redmine.example.com --command "<PASSWORD_MANAGER> <COMMAND>"

# example using 'pass'
$ redclock server add https://redmine.example.com --command "pass show redmine.example.com/api-key"
```

### Time Tracking

Once you have added at least one server, you can start tracking time. Redmine supports tracking time on both projects and issues.

```shell
$ redclock track start
```

The above command will ask you questions on which activity you are starting & which project/issue you are working on. You can specify those on the command line directly in case you do not like fuzzy selection.

Once you are done with your work, call:

```shell
$ redclock track stop
```

This will save the amount of hours you have worked on a project/issue in your Redmine instance.
