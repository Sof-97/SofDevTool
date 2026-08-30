# Personal Developer Toolbox

A private, local-only macOS application that gathers the small developer utilities its owner would otherwise look up online or open a larger developer application to use.

## Language

**Developer Toolbox**:
The application as a whole: a personal collection of focused developer utilities, not a replacement for an IDE, terminal, or full network client.
_Avoid_: DevTools replacement, developer platform

**Utility**:
A focused operation that generates, converts, formats, validates, compares, or inspects developer data entirely on the Mac.
_Avoid_: Tool, plugin, app

**Utility Operation**:
A successful application of a Utility to a complete input or generation request. Live edits that lead to one settled valid result constitute one Utility Operation; invalid and intermediate edits do not.
_Avoid_: Keystroke, edit event, attempt

**Utility Definition**:
The stable catalog identity and workspace entry point through which the Developer Toolbox knows a source-defined Utility.
_Avoid_: Plugin manifest, runtime extension

**Utility Registry**:
The authoritative catalog of Utility Definitions used by the main window and Utility Launcher for organization, search, and opening a Utility.
_Avoid_: Plugin registry, service locator

**Utility Operation Snapshot**:
A versioned, Utility-owned representation of one completed valid Utility Operation that can be offered to Utility History without deciding whether it will be retained.
_Avoid_: Autosave, edit event, persistence record

**Utility Workspace Session**:
The in-memory controls, input, diagnostics, and result for one opened Utility, retained while the Developer Toolbox is running even when another Utility is selected.
_Avoid_: History entry, saved workspace, screen

**Core Utility**:
A Utility that works without an account, network connection, hosted service, or paid API.
_Avoid_: Online tool, cloud tool

**Utility Launcher**:
The searchable interface opened from anywhere on macOS to find and open a Utility inside the Developer Toolbox.
_Avoid_: Command palette, Spotlight replacement

**Utility History**:
An on-device record of previous Utility inputs and results whose collection and retention the owner can configure.
_Avoid_: Cloud history, activity log

**Utility History Entry**:
A retained record of one Utility Operation, combining app-owned identity and capture time with the Utility-owned snapshot needed to preview or restore that operation.
_Avoid_: Autosave, workspace session, log event

**JWT Decoder**:
A Utility that decodes a JSON Web Token's readable segments for inspection without validating its signature, claims, trustworthiness, or fitness for use.
_Avoid_: JWT validator, JWT verifier

**Identifier Generator**:
A Utility for generating, validating, normalizing, and inspecting identifiers in the UUID, ULID, and KSUID format families. ULID and KSUID are identifier formats beside UUID, not UUID versions.
_Avoid_: UUID Utility, UUID type for ULID or KSUID
