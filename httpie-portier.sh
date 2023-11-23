#!/bin/bash
#Trigger Auth
http --follow -f POST http://localhost:8000/auth Accept:application/json email="email@test.com"

#Confirm
http -f POST https://broker.portier.io/confirm Accept:application/json session="your-session-code" code=144u6911e8td

#Verify
http -f POST http://localhost:8000/verify Accept:application/json id_token="your-token"
