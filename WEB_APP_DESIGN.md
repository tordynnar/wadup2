# Web Application Design for WADUP

# Purpose

The web application will be the primary way users create, edit, test, find and deploy WADUP modules.

# Visual Appearance

It will use the Catppuccin Macchiato dark color theme. The visual arrangement should be similar to VS Code.

# Features

## Authentication

Eventually, there will be proper authentication. For now, just have a single textbox to ask for their username and trust that they are who they say they are. Don't actually confirm their identity, that will be implemented later. If the user doesn't exit, create the user.

## Browse/search WADUP modules

From this screen, the user will see a paginated list of WADUP modules. They will be able to filter by:

- Just my modules (which will include published and unpublished modules from just the logged in user)
- All published modules (which will include all published modules by any users)
- Search the name of the module and all the code in the module

Modules will have the last modified date, author (user), and language listed next to each module.

From this screen there will be an option to create a new WADUP module. The user will pick between 3 languages - Rust, Go and Python. It will populate the new module with a template for the respective language with a basic example of how to create a module (demonstrating reading the input data and outputing metadata). When they create a new module, they will be taken to the view/edit screen to start editing it.

## View/edit WADUP module

When a user clicks on a module from the browser/search screen, they will be taken to an editor which lets the user view all of the code in the module. If the logged in user is the author of the module, they will be able to edit the module because they have permission to edit the module.

The editor will be the Monaco editor. It will have a file browser to the left of the editor to allow navigating through the files and folders in the module. If the user has permission to edit the module, they will be able to create new folders and files in the module (through the file browser).

Use 'web-tree-sitter' (or equivalent) to do language grammer and highlighting in the Monaco editor. It should support at least Rust, Go, Python and TOML formatting if possible.

There will be an option to build the WADUP module.

## Build System

When the user builds the module, it will show the debug output from the build process. The build process will happen on the backend inside a docker container. The container will be started for the build job, then remove and cleaned up immediately after the build is finished. The container should not run as the root user (for security).

The build system will pick the appropriate build scripts (Rust, Go, Python) depending on the language that was picked when creating the module.

## Publishing

When storing the module code, there will be two versions stored: the last published version (which is what is shown to other users), and the latest version that has been saved (but not published) by the author. The latest version will not be visible to other users. If the module has never been published, it will not be visible to anyone other than the author.

Users may only publish modules that were built successfully by the build system (producing a .wasm file). When it is publish, the compiled .wasm file will also be stored (no other artifacts from the build output need to be stored).

## Testing

There will be an option to test the module. The user will be able to upload sample data which will be only visible to the user who uploaded it. When the user has built the module, they will be able to pick one or more of the uploaded sample data to test against. It will then show each table of metadata that was output as well as the stdout and stderr.

This will require changing the WADUP CLI to support outputing in a format appropriate for testing (in addition to the existing output to Elasticsearch).

The testing will occur inside a docker container on the backend. The testing docker container will be started for the test, then removed and cleaned up when the test is finished. The container should not run as the root user (for security).

# Web App Deployment

The web app itself should be built as a docker image as well. Given the web app starts/stops docker containers, use DOOD to enable the web app to start/stop new containers.