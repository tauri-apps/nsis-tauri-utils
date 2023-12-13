#!/usr/bin/env node
// Copyright 2019-2022 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

/*
This script is solely intended to be run as part of the `covector publish` step to
check the latest version of a crate, considering the current minor version.
*/

const https = require("https");

const packageName = process.argv[2];
const packageVersion = process.argv[3];
const target = packageVersion.substring(0, packageVersion.lastIndexOf("."));

const options = {
  headers: {
    "Content-Type": "application/json",
    Accept: "application/json",
    Authorization: process.env["GITHUB_TOKEN"]
      ? `Bearer <${process.env["GITHUB_TOKEN"]}>`
      : null,
    "User-Agent": "tauri (https://github.com/tauri-apps/nsis-tauri-utils)",
  },
};

https.get(
  "https://api.github.com/repos/tauri-apps/nsis-tauri-utils/releases",
  options,
  (response) => {
    let chunks = [];
    response.on("data", function (chunk) {
      chunks.push(chunk);
    });

    response.on("end", function () {
      const data = JSON.parse(chunks.join(""));
      const versions = data.filter((t) =>
        t.tag_name.startsWith(`${packageName}-v${target}`)
      );
      console.log(
        versions.length
          ? versions[0].tag_name.replace(`${packageName}-v`, "")
          : "0.0.0"
      );
    });
  }
);
