-- NXM URL Handler Bridge for Crossover Mod Manager
-- This AppleScript receives nxm:// URLs and forwards them to the Tauri app

on open location this_URL
	try
		-- Log the received URL
		log "Received NXM URL: " & this_URL
		
		-- Write the URL to a file that the Tauri app will monitor
		set urlFile to (path to temporary items folder as text) & "nxm_url_queue.txt"
		set fileRef to open for access file urlFile with write permission
		write (this_URL & linefeed) to fileRef starting at eof
		close access fileRef
		
		-- Launch or bring to front the Tauri app
		tell application "Crossover Mod Manager"
			activate
		end tell
		
		display notification "NXM link received" with title "Crossover Mod Manager"
		
	on error errMsg
		log "Error: " & errMsg
		display dialog "Error handling NXM URL: " & errMsg buttons {"OK"} default button "OK"
	end try
end open location

on run
	display dialog "This is an NXM URL handler bridge for Crossover Mod Manager.

It should be set as the default handler for nxm:// URLs." buttons {"OK"} default button "OK"
end run
