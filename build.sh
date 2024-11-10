#!/bin/bash
trunk build --release
npx postcss dist/*.css --replace 