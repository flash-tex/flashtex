#!/usr/bin/env texlua
-- `l3build check` that runs no test: the tests a run would execute.
--
-- Usage (in a checkout directory, as run.py's --list does):
--   texlua /abs/path/l3build-list.lua check [-c CONFIG] -e ENGINE
--
-- This is l3build itself (l3build.lua, from TeX Live through kpathsea):
-- its startup, build.lua, the checkconfigs loop (each configuration in a
-- child process, which l3build starts by running this script again, since
-- it is arg[0]), check_engines (`Skipping unknown engine`), and check()'s
-- own test selection (testfiledir, test_order and test_types, includetests
-- and excludetests). Only two functions are replaced, once l3build-check.lua
-- has defined them: checkinit, which sets up the test area and unpacks the
-- sources (writing into the checkout), and runcheck, which runs one test.
-- So the transcript has run.py's `  name (i/n)` progress lines for exactly
-- the tests `l3build check` would run, and nothing is written.
--
-- One difference: check() also selects tests unpacked from the sources into
-- unpackdir, which only exist after checkinit's unpack. Without the unpack
-- that directory holds whatever the last run left (`l3build clean` empties
-- it). No suite in run.py's SUITE_DIRS unpacks a test; test_run.py checks
-- that the listing matches a run's transcript.

kpse.set_program_name("kpsewhich")

local real_require = require
function require(name)
  local module = real_require(name)
  if type(name) == "string" and name:match("l3build%-check%.lua$") then
    function checkinit() return 0 end
    function runcheck() return 0 end
  end
  return module
end

dofile(kpse.lookup("l3build.lua"))
