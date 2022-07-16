# Minecraft Server Locator

This tool takes the output of masscan and attempts to use the Minecraft server status query API to gather and save details about a suspected Minecraft server.

You will need a lot of `xsv`, `sed`, `awk`, and other "glue programs" to use this properly.

Eventually I will make this application fully integrated.

On a slow connection (<50Mbps), this tool can query 2000 servers per hour doing the following:

1. Test to see if the server is actually online
2. Test to see if the server is actually a Minecraft server
3. Gather the server details (version, max players, motd, etc.)
4. Save them as a CSV
