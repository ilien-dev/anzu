# Anzu main window

Mode: Operate. The user comes to convert their books; the screen is a tool.

## Direction contract

THESIS: A bright working desk: the books are on shelves by what they still need, and a
side panel says what is going to happen before the button does it. While a run goes on,
the window becomes a list of steps that says what Anzu is doing now and what comes next.

OWN-WORLD: “Coloured paper”, chosen by the user (proposal 2A). It contributes four things
and only those: Space Grotesk headings over Inter text, soft corners (10 px controls,
14 px cards), four state pastels used as fills with ink on top (peach DRM, lilac unlocked,
mint EPUB, butter selected or running), and drawn line icons beside every action. Light
theme: cool white #F4F5F7, white panels, ink #15161A, the main action ink. Dark theme:
night #0E0F12, panels #16171B, light ink #ECEDEF, the main action butter #FFF27A. The
header is a band of dark ink in both themes. Pastels never colour text.

STORY: The user sees whether the system is ready, recognises their books by title on the
shelves, ticks the ones they want, reads in the side panel what will happen to each, and
presses a button that says it.

FIRST VIEWPORT: Header band with the tricolour logo and a cog that opens a floating
settings panel (theme and language as icon segments, version and “Check for updates”).
Below it, on the left: the status line with the Kindle, tool and checksum facts as chips
and “Check again” at its end; the heading “Your shelf” with “Refresh list” (offline, it
says how many books are new) and the sort order (title, ASIN, size); then the shelves, each
with its pastel tile, its count and “Mark all”: To unlock, Ready for EPUB, and In EPUB,
folded by default. Each book is a card with a cover in its state pastel (a lock, or the
initials of its title), its title or ASIN, its size and a checkbox. On the right, the side
panel: the chosen books with what will be done to each, the destination folder with
“Change folder”, the note that Kindle will be closed when something needs unlocking, the
notice for books that already have an EPUB, and the main action at its foot.

PROGRESS: One centred column (700 px at most) with the steps of the run: the preparation,
unlock n books, convert n books and save to your folder, with "Step k of n". Each step is a
numbered circle that turns butter while it runs and mint when done. Under the conversion
step, "k / n" on the right and every book as a tile: quiet while it waits, butter while it
is worked on, swelling into mint when its EPUB is written, with a peach edge when it fails;
up to 4 books convert at once.
Saving runs for the half second the pacer holds the end; when done its title says "N EPUBs
are ready", its detail names the one folder, and it holds "Show in folder" (plus "Open
EPUB" for one book, or "See the N books": each book with its cover, its three-stage track
and "Open EPUB" or what went wrong). Then a note when some failed,
"Convert more books" and the log behind a link. The conversions of a run last at least
1.3 s + 0.12 s per book, 2.6 s at most, and nothing is shown done before it is. The header
says "Working" or "Finished".

SIGNATURE: Each book's state is a pastel: its cover on the shelf and its three-stage track in
the list of a finished run, DRM → unlocked → EPUB, fill in the same colours.

REFUSALS: No terminal costume. No emoji or text glyphs as icons: every icon is drawn. No
pastel as the colour of text.
