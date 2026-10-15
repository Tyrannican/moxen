# 0.5.0

* Removed `install` CLI argument
    * This is now handled by the `update` CLI argument
* Renamed `uninstall` CLI argument to `remove`
    * Sounds cleaner
* Removed the `updated` table in the DB
    * This was silly and unnecessary

# 0.4.0

* BREAKING: Moved from disk-based registry to SQLite3 registry
* BREAKING: Removed old disk-related registry items.
