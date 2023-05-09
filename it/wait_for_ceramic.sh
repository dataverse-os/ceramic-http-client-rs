#!/usr/bin/env bash

while [ $(curl -s -o /dev/null -I -w "%{http_code}" "http://127.0.0.1:7071/api/v0/node/healthcheck") -ne "200" ]; do
  sleep 1
done