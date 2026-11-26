#!/usr/bin/env bash
# scripts/preflight.sh — local reproduction of the CI gates before `git push`.
# Every command is the one CI runs. A step this file does not cover is a step
# that can only fail remotely — when a workflow changes, regenerate this file
# in the same commit (checks no workflow runs go in scripts/preflight.local.sh).
# Run `--quick` before every push.
#
# usage: scripts/preflight.sh [--quick]   (--quick skips the test / bench suites)
# generator rev 13c60fc63085b2e65d558ab66e5fb9ed5fbdedd6
set -euo pipefail
cd "$(dirname "$0")/.."
# --- heavy slot (begin) ---
# ALICE_HEAVY_SLOTS (default 2, 1..16) heavy runs at a time on this machine. Slot k is a lock the kernel holds (flock
# on $ALICE_HEAVY_SLOT_DIR/lock.<k>, default $HOME/.cache/alice-heavy-slot) for as long as its holder lives. A small
# keeper process (perl) takes it and exits within a second of the holder's death, so a killed holder frees the slot
# and no child of the holder keeps it. Waiters queue: each keeper leaves a ticket in queue/ named
# `<start time %017.6f>-<pid>` for a `work` take and `0<start time>-<pid>` for a `land` take (a work ticket older
# than ALICE_HEAVY_SLOT_AGE_SECS, default 9000, renames itself to the land form), and only the first live ticket in
# the names' sort order may take a free slot: land before work, then by arrival. Copies of this block from before
# the classes write work tickets and use the same sort, so old and new waiters agree on the order. A ticket whose
# keeper is gone (a crash) is skipped and removed. A priority file ($ALICE_HEAVY_SLOT_DIR/priority: one label per
# line, top first; `#` starts a comment) ranks the waiters whose label is a line or starts with "<line>-" before all
# others: their tickets are named `00<rank, 2 digits><0 land | 1 work><start time>-<pid>`, so they sort first for
# every copy of this block, an older one too (it sorts the names). A waiter re-reads the file each poll and renames
# its ticket when its rank changes; an unlisted label keeps the names above; a line that is not a label is ignored
# with a warning; a rank never takes a held slot. The light slot (heavy_slot_acquire <label> <tool> <class> light)
# is the same with light.0, queue-light/, info-light.0 and a `light-keeper` title. A waiter prints its place and its wait once a minute. The keeper of slot k names
# itself `alice-heavy-slot keeper slot=<k> holder=<pid> dirhex=<the dir's bytes in hex> end` (its process title,
# which no file cleanup removes; the dir in hex so no dir can end the title early), and the holder of slot k is read
# from there (only from a perl process, and only while the holder lives); `info.<k>` only adds the holder's label and tool, and a
# release removes it only while it names the releasing shell. A wait whose place has not moved for ALICE_HEAVY_SLOT_STALL_SECS (default 10800) or past ALICE_HEAVY_SLOT_TIMEOUT in all (default 28800) fails; the holder's own nested runs
# (ALICE_HEAVY_SLOT_HELD) run inside its slot.
heavy_slot_dir() { printf '%s\n' "${ALICE_HEAVY_SLOT_DIR:-$HOME/.cache/alice-heavy-slot}"; }
# Disk admission (disk_reserve.py) is on for every heavy and short take; no environment name turns it off. A test
# turns it off for its own slot dir only, with a file `disk-admission-off` in that dir. A slot dir under the account's
# home (from the password database, whatever HOME says) refuses the file: the machine's queue (.cache/alice-heavy-slot
# there) and the Air's receipt queue (remote-receipts/slot there) always admit by disk; a test's dir is a temporary dir.
# Prints nothing but a refusal; returns 0 = admission on, 1 = off (a test's dir), 2 = refused.
heavy_slot_disk_admits() {
  local d="$1" home
  [ -e "$d/disk-admission-off" ] || return 0
  home=$(perl -e 'print((getpwuid($<))[7])')
  local real
  real=$(cd "$d" 2>/dev/null && pwd -P) || real=$d
  home=$(cd "$home" 2>/dev/null && pwd -P) || home=""
  case "$real/" in
    "$home"/*)
      echo "heavy slot: $d/disk-admission-off is refused in a slot dir under the account's home (remove it; disk admission stays on)" >&2
      return 2 ;;
  esac
  [ -n "$home" ] || { echo "heavy slot: the account's home is unknown; $d/disk-admission-off is refused" >&2; return 2; }
  return 1
}
heavy_slot_count() {
  case "${ALICE_HEAVY_SLOTS:-2}" in
    [1-9] | 1[0-6]) printf '%s\n' "${ALICE_HEAVY_SLOTS:-2}" ;;
    *) echo "heavy slot: ALICE_HEAVY_SLOTS must be an integer from 1 to 16 (got '${ALICE_HEAVY_SLOTS:-}')" >&2; return 1 ;;
  esac
}
# whether $1 is a perl process (a keeper): its title is its own, so the executable is checked (macOS keeps it as
# the accounting name `ucomm`; Linux sets comm from the title, so the executable's path is read there)
heavy_slot_is_perl() {
  local exe
  if [ -e "/proc/$1/exe" ]; then
    exe=$(readlink "/proc/$1/exe" 2>/dev/null) || return 1
    case "${exe##*/}" in perl*) return 0 ;; *) return 1 ;; esac
  fi
  [ "$(ps -o ucomm= -p "$1" 2>/dev/null | tr -d ' ')" = perl ]
}
# "<k> <holder pid>" of every slot of $1 held by a live keeper (from the keepers' process titles: a perl process
# whose title names this dir, while its holder lives). A slot whose keeper has no title (a keeper of a copy of this
# block from before the titles: an older generated preflight, or a run started before the update) is read from
# info.<k> while its holder lives and lock.<k> is held
heavy_slot_live() {
  local d=$1 hex titled="" f k h p
  hex=$(printf '%s' "$d" | od -An -tx1 | tr -d ' \n')
  while read -r p k h; do
    heavy_slot_is_perl "$p" || continue
    kill -0 "$h" 2>/dev/null || continue
    titled="${titled:+$titled
}$k $h"
  done < <(ps -A -ww -o pid=,command= 2>/dev/null | awk -v hex="$hex" '
    { if (match($0, /alice-heavy-slot keeper slot=[0-9]+ holder=[0-9]+ dirhex=[0-9a-f]+ end/)) {
        split(substr($0, RSTART, RLENGTH), w, /[ =]/)
        if (w[8] == hex) print $1, w[4], w[6] } }')
  {
    [ -n "$titled" ] && printf '%s\n' "$titled"
    for f in "$d"/info.*; do
      k=${f##*/info.}
      case "$k" in '' | *[!0-9]*) continue ;; esac
      printf '%s\n' "$titled" | awk -v k="$k" '$1 == k { found = 1 } END { exit !found }' && continue
      h=$(sed -n 1p "$f" 2>/dev/null)
      case "$h" in '' | *[!0-9]*) continue ;; esac
      kill -0 "$h" 2>/dev/null || continue
      perl -MFcntl=:flock -e 'open(my $f, ">>", $ARGV[0]) or exit 1; exit(flock($f, LOCK_EX | LOCK_NB) ? 1 : 0)' \
        "$d/lock.$k" 2>/dev/null || continue
      printf '%s %s\n' "$k" "$h"
    done
  } | sort -n
}
# "label pid (tool)" of every held slot, joined with ", " (the label and tool from info.<k> while it names the
# holder, `unlabelled` otherwise)
heavy_slot_holders() {
  local d=$1 k h f out=""
  while read -r k h; do
    f="$d/info.$k"
    if [ "$(sed -n 1p "$d/info.short.0" 2>/dev/null)" = "$h" ]; then f="$d/info.short.0"; fi
    if [ "$(sed -n 1p "$f" 2>/dev/null)" = "$h" ]; then
      out="${out:+$out, }$(sed -n 3p "$f") pid $h ($(sed -n 4p "$f"))"
    else
      out="${out:+$out, }unlabelled pid $h (info.$k missing)"
    fi
  done < <(heavy_slot_live "$d")
  printf '%s\n' "${out:-nobody}"
}
heavy_slot_acquire() {
  local label=$1 tool=$2 class=${3:-${ALICE_HEAVY_SLOT_CLASS:-work}} pool=${4:-heavy} d n k f line err \
    limit="${ALICE_HEAVY_SLOT_TIMEOUT:-}" stall="${ALICE_HEAVY_SLOT_STALL_SECS:-10800}" age="${ALICE_HEAVY_SLOT_AGE_SECS:-9000}" queue=queue lock=lock. \
    info=info. title=keeper
  case "$class" in land | work) ;; *) echo "heavy slot: the class must be land or work (got '$class')" >&2; return 2 ;; esac
  # a waiter gives up when its place has not moved up for ALICE_HEAVY_SLOT_STALL_SECS (default 3 h: no holder finished
  # in that time), or past ALICE_HEAVY_SLOT_TIMEOUT in all (default 8 h, far above the longest wait seen, 2.7 h on
  # 2026-11-07); a run still moving up is never dropped for its total wait alone before that (a 2 h total limit dropped
  # three that day). HEAVY_SLOT_WAIT_END says which (stalled / timeout) for the take record
  if [ -z "$limit" ]; then limit=28800; fi
  case "$stall" in '' | *[!0-9]*) echo "heavy slot: ALICE_HEAVY_SLOT_STALL_SECS must be a whole number of seconds" >&2; return 2 ;; esac
  HEAVY_SLOT_WAIT_END=""
  # HEAVY_SLOT_RESERVE_KEY (the caller's: repository, commit, label): a waiter restarted within
  # ALICE_HEAVY_SLOT_RESERVE_SECS (default 1800) of its predecessor's death gets that place back
  local rkey="" hold="${ALICE_HEAVY_SLOT_RESERVE_SECS:-1800}"
  case "$hold" in '' | *[!0-9]*) echo "heavy slot: ALICE_HEAVY_SLOT_RESERVE_SECS must be a whole number of seconds" >&2; return 2 ;; esac
  if [ -n "${HEAVY_SLOT_RESERVE_KEY:-}" ]; then
    rkey=$(printf '%s|%s|%s' "$HEAVY_SLOT_RESERVE_KEY" "$pool" "$class" | shasum -a 256 2>/dev/null | cut -c1-40)
  fi
  case "$age" in '' | *[!0-9]*) echo "heavy slot: ALICE_HEAVY_SLOT_AGE_SECS must be a whole number of seconds" >&2; return 2 ;; esac
  d=$(heavy_slot_dir)
  n=$(heavy_slot_count) || return 1
  if [ -n "${ALICE_HEAVY_SLOT_HELD:-}" ] && kill -0 "$ALICE_HEAVY_SLOT_HELD" 2>/dev/null; then
    heavy_slot_live "$d" | awk -v h="$ALICE_HEAVY_SLOT_HELD" '$2 == h { found = 1 } END { exit !found }' && return 0
  fi
  if [ "$pool" = light ]; then
    # one light slot; a light take inside a light holder runs inside it
    if [ -n "${ALICE_HEAVY_SLOT_LIGHT_HELD:-}" ] && kill -0 "$ALICE_HEAVY_SLOT_LIGHT_HELD" 2>/dev/null; then return 0; fi
    n=1 queue=queue-light lock=light. info=info-light. title=light-keeper
  elif [ "$pool" = short ]; then
    # one short slot, its own queue; a nested take (the preflight inside) adopts it through ALICE_HEAVY_SLOT_HELD above
    n=1 queue=queue-short lock=short. info=info.short.
  fi
  # disk reservations (disk_reserve.py): heavy and short takes, unless a test's slot dir turns them off
  local dscript="" dkey="${HEAVY_SLOT_DISK_KEY:-$label}" dadm=0
  if [ "$pool" != light ]; then
    heavy_slot_disk_admits "$d" || dadm=$?
    [ "$dadm" = 2 ] && return 2
  fi
  if [ "$pool" != light ] && [ "$dadm" = 0 ]; then
    dscript="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/disk_reserve.py"
    [ -f "$dscript" ] || dscript=""
  fi
  if ! err=$(mkdir -p "$d/$queue" 2>&1); then
    echo "heavy slot: cannot create $d: ${err:-not writable}" >&2
    return 1
  fi
  for k in $(seq 0 $((n - 1))); do
    if ! err=$({ : >> "$d/$lock$k"; } 2>&1); then
      echo "heavy slot: cannot create $d: ${err:-not writable}" >&2
      return 1
    fi
  done
  # (the keeper closes fd 7, the GPU lane's lock, before it starts: `<(exec 7>&-; exec …)`, since bash 3.2 leaves the
  # descriptor open for `<(exec perl … 7>&-)`, measured 2026-11-25)
  exec 8< <(exec 7>&-; exec perl -MFcntl=:flock -MTime::HiRes=time,sleep -e '
    my ($dir, $n, $limit, $stall, $class, $age, $qname, $lock, $title, $label, $rkey, $hold, $dscript, $dkey, $pool, $infop) = @ARGV;
    my $orig = $class;  # (a promoted work waiter ranks as land but is still a work take for the one-work rule)
    # disk (disk_reserve.py admit): the head of the queue takes a slot only when the disk can hold its build next to
    # what the running takes will still grow; asked at most every 5 s while first (it reads df and the small files
    # of the trackers, no du), so a take that ends frees its reservation for the next one within seconds
    my ($dok, $dlast, $dwhy) = (1, 0, "");
    my $parent = getppid(); $| = 1;
    my $q = "$dir/$qname";
    my $stamp = sprintf("%017.6f", time);
    # a waiter that died keeps its place for $hold s: a new waiter of the same key (repository, commit, label; the
    # caller hashes it) whose previous waiter is gone takes the old arrival time, so it sorts where it stood
    my $rdir = "$dir/reserve-$qname"; my $res = ""; my $touched = 0;
    if (defined $rkey && $rkey =~ /^[0-9a-f]{40}$/) {
      mkdir $rdir;
      if (opendir(my $rd, $rdir)) {
        for my $f (grep { /^[0-9a-f]{40}$/ } readdir $rd) { my $m = (stat("$rdir/$f"))[9]; unlink "$rdir/$f" if defined $m && time - $m > $hold + 60 }
        closedir $rd;
      }
      $res = "$rdir/$rkey";
      if (open(my $rf, "<", $res)) {
        my ($s, $p) = split " ", (scalar(<$rf>) // ""); close $rf;
        my $m = (stat($res))[9];
        if (defined $s && $s =~ /^\d+\.\d+$/ && $p && $p =~ /^\d+$/ && !kill(0, $p) && defined $m && time - $m <= $hold) { $stamp = $s }
      }
      if (open(my $wf, ">", "$res.tmp")) { print $wf "$stamp $$\n"; close $wf; rename "$res.tmp", $res }
      $touched = time;
    }
    my %warned;
    my $name = sub {
      my $r;
      if (defined $label && open(my $pf, "<", "$dir/priority")) {
        my ($i, $ln) = (0, 0);
        while (my $line = <$pf>) {
          $ln++; chomp $line; (my $l = $line) =~ s/#.*//; $l =~ s/^\s+|\s+$//g;
          next if $l eq "";
          if ($l !~ /^[A-Za-z0-9._-]+$/) {
            print STDERR "heavy slot: priority file line $ln ignored (not a label): $line\n" unless $warned{$ln}++;
            next;
          }
          if ($label eq $l || index($label, "$l-") == 0) { $r = $i; last }
          last if ++$i > 99;
        }
        close $pf;
      }
      my $landish = $class =~ /^land/;
      return defined $r ? sprintf("00%02d%s%s-%d", $r, $landish ? "0" : "1", $stamp, $$) : ($landish ? "0" : "") . "$stamp-$$";
    };
    my $me = $name->();
    my $ticket = "$q/$me";
    open(my $tf, ">", $ticket) or do { print "error $!\n"; exit 2 }; print $tf ($label // ""), "\n"; close $tf;
    # the commit this take is for, beside the ticket: `.meta-<pid>` (JSON: label, pid, start, top = the checkout, sha;
    # `end` written when the keeper exits). scripts/land_reserve.py reads it: a waiting or running take keeps the
    # landing reservation of its holder alive (2026-11-24: a reservation sized for a 2 h CI expired while its local receipt
    # waited 10 h here). A dotfile keyed by pid and never renamed, so the ticket sort and the status readers (which
    # read digit-led names) skip it; a take outside a git checkout writes none; a meta older than a day goes
    my $metaf = "$q/.meta-$$";
    my $metaj = "";
    {
      my $js = sub { my $v = shift; $v =~ s/([\\"])/\\$1/g; $v =~ s/[\x00-\x1f]//g; "\"$v\"" };
      my @g = `git rev-parse --show-toplevel HEAD 2>/dev/null`; chomp @g;
      if (@g == 2 && $g[1] =~ /^[0-9a-f]{40}$/) {
        $metaj = "{\"label\": " . $js->($label // "") . ", \"pid\": $$, \"start\": " . time . ", \"top\": "
          . $js->($g[0]) . ", \"sha\": \"$g[1]\"";
        if (open(my $mf, ">", "$metaf.tmp")) { print $mf "$metaj}\n"; close $mf; rename("$metaf.tmp", $metaf) }
      }
      if (opendir(my $md, $q)) {
        for my $f (grep { /^\.meta-\d+$/ } readdir $md) {
          my $m = (stat("$q/$f"))[9]; unlink "$q/$f" if defined $m && time - $m > 86400;
        }
        closedir $md;
      }
    }
    # a waiter the disk holds (its own admission failed) says so beside its ticket: `.disk-<ticket>`, the reason,
    # refreshed at each ask; a waiter behind waiters that are all disk-held may take a free slot when its own
    # admission passes (2026-11-10: a head reserving 30.8 GiB of 25.5 free held 5 fitting waiters for 3 h); the head
    # keeps its place: it clears its marker as soon as the disk fits, and then nobody behind it may pass
    my $mark = sub { "$q/.disk-$me" };
    # the keeper of a ticket: its pid alive and that process started no later than the ticket file was written (+2 s:
    # ps gives whole seconds; the keeper writes its ticket once, after it starts, and a rename keeps the time); a later
    # process that took the pid of a dead keeper is not it. Not the time in the name: a restarted waiter writes its old
    # time into a new ticket. A ticket gone meanwhile, or a start ps cannot give, falls back to the pid alone
    my $keeper = sub {
      my ($p, $t) = @_;
      my $stamp = (stat("$q/$t"))[9];
      return 1 unless defined $stamp;
      my $et = `ps -o etime= -p $p 2>/dev/null`; $et =~ s/\s+//g;
      return 1 if $et eq "";
      my $days = ($et =~ s/^(\d+)-//) ? $1 : 0;
      my $secs = 0; $secs = $secs * 60 + $_ for split /:/, $et;
      return time - ($secs + $days * 86400) <= $stamp + 2;
    };
    # one work take at a time on the heavy slots: true while a held slot (its lock held by its keeper, its info
    # naming class `work`) is a work take; only a work waiter asks. The lock, not the pid: a killed holder that its
    # parent has not reaped still answers kill 0, while its keeper (and the lock) is gone within a second
    my @fh;  # (the lock file of each slot, opened below; declared here for $work_busy)
    # (only while a slot is free: with every slot held the waiter waits as before, and marks nothing)
    my $work_busy = sub {
      return 0 unless defined $pool && $pool eq "heavy" && $orig eq "work";
      my ($free, $work) = (0, 0);
      for my $k (0 .. $n - 1) {
        if (flock($fh[$k], LOCK_EX | LOCK_NB)) { flock($fh[$k], LOCK_UN); $free = 1; next }  # (nobody holds slot k)
        # (a held slot whose info is not written yet, or names a holder that is gone, counts as work: two work
        # takes asking at once would otherwise both take a slot before either wrote its info)
        my @l;
        if (open(my $i, "<", "$dir/$infop$k")) { @l = <$i>; close $i; chomp @l }
        $work = 1 if @l < 5 || $l[0] !~ /^\d+$/ || !kill(0, $l[0]) || $l[4] eq "work";
      }
      return $free && $work;
    };
    my $wmark = sub { "$q/.work-$me" };
    my ($wb, $wlast) = (0, 0);
    my $clear = sub { unlink $mark->() };
    END {
      unlink $ticket if defined $ticket; if (defined $me) { unlink "$q/.disk-$me"; unlink "$q/.work-$me" }
      if (defined $metaj && $metaj ne "" && open(my $mf, ">", "$metaf.tmp")) {
        print $mf "$metaj, \"end\": " . time . "}\n"; close $mf; rename("$metaf.tmp", $metaf);
      }
    }
    # heavy_slot_release ends a holding keeper with TERM: exit so that END runs (the meta gets its end; a waiting
    # keeper sent TERM also clears its ticket now instead of leaving it to the sweep)
    $SIG{TERM} = sub { exit 0 };
    for my $k (0 .. $n - 1) { open($fh[$k], ">>", "$dir/$lock$k") or do { print "error $!\n"; exit 2 } }
    print "pid $$\n";
    my $start = time; my $note = 0; my $best; my $moved = time;
    while (1) {
      my $want = $name->();
      if ($want ne $me && rename($ticket, "$q/$want")) { $clear->(); unlink $wmark->(); $me = $want; $ticket = "$q/$me" }
      opendir(my $dh, $q) or do { print "error $!\n"; exit 2 };
      my @live;
      my @names = readdir $dh;
      for my $t (sort grep { /^\d/ } @names) {
        my ($p) = $t =~ /-(\d+)$/;
        if ($p && ($p == $$ || (kill(0, $p) && $keeper->($p, $t)))) { push @live, $t } else { unlink "$q/$t" }
      }
      # a `.disk-<ticket>` marker whose ticket is gone (its keeper was killed before its END ran) goes with it: it
      # held nothing, but read as a waiter (2026-11-16: one from 11-15 02:26 shown as a 26 h wait)
      my %held = map { $_ => 1 } @live, $me;
      for my $mk (grep { /^\.(?:disk|work)-/ } @names) {
        my ($t) = $mk =~ /^\.(?:disk|work)-(.+?)(?:\.tmp)?$/;
        unlink "$q/$mk" if defined $t && !$held{$t};
      }
      closedir $dh;
      my ($place) = grep { $live[$_] eq $me } 0 .. $#live;
      # may ask for a slot: the head, or a waiter every one before which is held by the disk (a marker fresher than
      # 15 s; a copy of this block from before the markers writes none, so nobody passes it)
      # (or, for a land waiter only, held by the one-work rule: a `.work-<ticket>` marker fresher than 15 s; work
      # waiters keep their order among themselves)
      my $eligible = defined $place && ($place == 0 || !grep {
        my $md = (stat("$q/.disk-$live[$_]"))[9]; my $mw = (stat("$q/.work-$live[$_]"))[9];
        !(($dscript ne "" && defined $md && time - $md <= 15) || ($orig ne "work" && defined $mw && time - $mw <= 15)) } 0 .. $place - 1);
      # a work waiter behind waiters that are all held by the disk or the one-work rule may not pass them, but it is
      # held by the same rule: it asks whether work is busy and marks itself so, never taking a slot (the work order
      # is kept); without the marker the land waiters behind it could not pass (2026-11-24: two work waiters at the
      # head held three land takes ~6 h with a slot free)
      my $wonly = !$eligible && defined $place && $place > 0 && $orig eq "work" && !grep {
        my $md = (stat("$q/.disk-$live[$_]"))[9]; my $mw = (stat("$q/.work-$live[$_]"))[9];
        !(($dscript ne "" && defined $md && time - $md <= 15) || (defined $mw && time - $mw <= 15)) } 0 .. $place - 1;
      if (!$eligible && !$dok) { ($dok, $dlast) = (1, 0); $clear->() }  # (passed by: asks again when it may)
      if ($eligible && defined $dscript && $dscript ne "" && time - $dlast >= 5) {
        $dwhy = `python3 "$dscript" admit "$dir" "$dkey" 2>&1`; $dok = ($? == 0); $dlast = time; chomp $dwhy;
        if ($dok) { $clear->() } elsif (open(my $mf, ">", $mark->() . ".tmp")) { print $mf "$dwhy\n"; close $mf; rename $mark->() . ".tmp", $mark->() }
        $moved = time if !$dok;  # (waiting for disk is not a stalled queue: the head does not give up for it)
      }
      if (($eligible && $dok) || $wonly) {
        $wb = $work_busy->();
        if (!$wb) { unlink $wmark->() }
        elsif (time - $wlast >= 5 && open(my $wf, ">", $wmark->() . ".tmp")) {
          print $wf "one work take at a time\n"; close $wf; rename $wmark->() . ".tmp", $wmark->(); $wlast = time;
        }
      } else { $wb = 0; unlink $wmark->() }
      if ($eligible && $dok && !$wb) {
        for my $k (0 .. $n - 1) {
          next unless flock($fh[$k], LOCK_EX | LOCK_NB);
          $clear->(); unlink $ticket; undef $ticket; unlink $res if $res ne "";
          $0 = "alice-heavy-slot $title slot=$k holder=$parent dirhex=" . unpack("H*", $dir) . " end";
          print "held $k\n"; close(STDOUT);
          sleep 1 while getppid() == $parent;
          exit 0;
        }
      }
      my $w = int(time - $start);
      if ($class eq "work" && $w >= $age) { $class = "land (promoted)" }  # (renamed at the next poll)
      if (defined $place && (!defined $best || $place < $best)) { $best = $place; $moved = time }
      if ($w >= $limit) { unlink $res if $res ne ""; $clear->(); print "timeout\n"; exit 1 }
      if (time - $moved >= $stall) { unlink $res if $res ne ""; $clear->(); print "stalled\n"; exit 1 }
      if ($res ne "" && time - $touched >= 30) { utime(undef, undef, $res); $touched = time }
      if ($w >= $note) { printf "wait %d %d %s%s%s\n", ($place // 0) + 1, $w, $class, $dok ? "" : " disk", $wb ? " worklimit" : ""; $note += 60 }
      sleep 0.2;
      exit 3 if getppid() != $parent;
    }
  ' "$d" "$n" "$limit" "$stall" "$class" "$age" "$queue" "$lock" "$title" "$label" "$rkey" "$hold" "$dscript" "$dkey" "$pool" "$info")
  while IFS= read -r line <&8; do
    case "$line" in
      "pid "*) HEAVY_SLOT_KEEPER=${line#pid } ;;
      "wait "*)
        set -- $line
        case "$2" in 1) k=1st ;; 2) k=2nd ;; 3) k=3rd ;; *) k="$2th" ;; esac
        if [ "${@: -1}" = worklimit ]; then
          echo "heavy slot: $k in the queue, waiting ($3 s): one work take at a time, held by $(heavy_slot_holders "$d")" >&2
        elif [ "${6:-${5:-}}" = disk ]; then
          echo "heavy slot: $k in the queue, waiting for disk space ($3 s): $(python3 "$dscript" admit "$d" "$dkey" 2>&1)" >&2
        else
          echo "heavy slot: waiting ($k in the queue, $3 s) for $(heavy_slot_holders "$d"); ${pool} slot, as $4${5:+ $5}" >&2
        fi
        ;;
      "held "*)
        exec 8<&-
        HEAVY_SLOT_INDEX=${line#held }
        HEAVY_SLOT_INFO="$d/$info$HEAVY_SLOT_INDEX"
        printf '%s\n%s\n%s\n%s\n%s\n' "$$" "$(date +%s)" "$label" "$tool" "$class" > "$HEAVY_SLOT_INFO"
        # the take's reservation, kept by a tracker that samples its target dirs and records the peak when it ends
        if [ -n "$dscript" ]; then
          nohup python3 "$dscript" track "$d" "$dkey" "$$" ${HEAVY_SLOT_DISK_ROOTS:-$PWD} </dev/null >/dev/null 2>&1 7>&- &
        fi
        if [ "$pool" = light ]; then
          ALICE_HEAVY_SLOT_LIGHT_HELD=$$
          export ALICE_HEAVY_SLOT_LIGHT_HELD
        else
          ALICE_HEAVY_SLOT_HELD=$$
          export ALICE_HEAVY_SLOT_HELD
        fi
        return 0
        ;;
      timeout)
        exec 8<&-
        HEAVY_SLOT_WAIT_END=timeout
        echo "heavy slot: waited ${limit}s for $(heavy_slot_holders "$d"); giving up (ALICE_HEAVY_SLOT_TIMEOUT=$limit)" >&2
        return 1
        ;;
      stalled)
        exec 8<&-
        HEAVY_SLOT_WAIT_END=stalled
        echo "heavy slot: no place gained for ${stall}s behind $(heavy_slot_holders "$d"); giving up (ALICE_HEAVY_SLOT_STALL_SECS=$stall)" >&2
        return 1
        ;;
      "error "*)
        exec 8<&-
        echo "heavy slot: cannot open $d/lock: ${line#error }" >&2
        return 1
        ;;
    esac
  done
  exec 8<&-
  echo "heavy slot: the lock keeper stopped (is perl on PATH?)" >&2
  return 1
}
heavy_slot_release() {
  local d f
  d=$(heavy_slot_dir)
  if { [ "${ALICE_HEAVY_SLOT_HELD:-}" = "$$" ] || [ "${ALICE_HEAVY_SLOT_LIGHT_HELD:-}" = "$$" ]; } &&
    [ -n "${HEAVY_SLOT_KEEPER:-}" ]; then
    # the info first: once the keeper is gone the next holder writes its own. Only while it names this shell: when
    # the keeper died first (`pkill -P` on the holder), the slot may already be another holder's, with its info
    f=${HEAVY_SLOT_INFO:-$d/info.${HEAVY_SLOT_INDEX:-}}
    if [ "$(sed -n 1p "$f" 2>/dev/null)" = "$$" ]; then
      rm -f "$f"
    fi
    kill "$HEAVY_SLOT_KEEPER" 2>/dev/null || true
    HEAVY_SLOT_KEEPER=
  fi
}
# takes: one line per take in $ALICE_HEAVY_SLOT_DIR/takes.tsv (UTC time, lane, label, repository, seconds waited,
# seconds held, exit, peak MiB of the whole tree, note). The take that records is the outermost one (its pid is
# HEAVY_SLOT_TAKE_ROOT, set when it gets its slot): it records at its exit, or the watcher or the short wall records
# for it when they kill its tree (a kill -9 runs no exit trap). The peak is the watcher's: the RSS (or footprint) of
# the take's process and every descendant, kept in HEAVY_SLOT_PEAK_FILE as the largest seen. A record that cannot be
# written changes nothing about the take.
heavy_slot_record_take() {
  local rc=$1 peak note=${2:-}
  [ -e "${HEAVY_SLOT_PEAK_FILE:-/nonexistent}.done" ] && return 0
  peak=$(cat "${HEAVY_SLOT_PEAK_FILE:-/dev/null}" 2>/dev/null) || peak=0
  case "$peak" in '' | *[!0-9]*) peak=0 ;; esac
  [ -n "$note" ] || note=$(cat "${HEAVY_SLOT_PEAK_FILE:-/nonexistent}.note" 2>/dev/null) || note=""
  { printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$(date -u +%FT%TZ)" "${HEAVY_SLOT_TAKE_LANE:-}" \
      "${HEAVY_SLOT_TAKE_LABEL:-}" "${HEAVY_SLOT_TAKE_REPO:-}" "${HEAVY_SLOT_TAKE_WAITED:-}" \
      "$(( $(date +%s) - ${HEAVY_SLOT_TAKE_T0:-$(date +%s)} ))" "$rc" "${peak:-}" "$note" \
      >> "$(heavy_slot_dir)/takes.tsv"; } 2>/dev/null || true
  [ -z "${HEAVY_SLOT_PEAK_FILE:-}" ] || : > "$HEAVY_SLOT_PEAK_FILE.done" 2>/dev/null || true
}
# a row for a take that never held a slot (refused at entry, or its wait timed out): lane label repo waited exit note
heavy_slot_record_row() {
  { printf '%s\t%s\t%s\t%s\t%s\t0\t%s\t0\t%s\n' "$(date -u +%FT%TZ)" "$1" "$2" "$3" "$4" "$5" "$6" \
      >> "$(heavy_slot_dir)/takes.tsv"; } 2>/dev/null || true
}
# the process groups a take's processes run in, kept across samples: a child forked twice (its parent gone) leaves
# the tree but keeps its group, so its memory still counts and a cap kill still reaches it. The group of the caller
# (the watcher's own) is never followed: it holds other work. A process that starts a session of its own (setsid)
# leaves both and is not seen. The result is in HEAVY_SLOT_GROUP_TREE (a variable, so the groups persist in the caller).
HEAVY_SLOT_GROUPS=""
heavy_slot_group_tree() {
  local pg own out
  HEAVY_SLOT_GROUP_TREE=$1
  pg=$(ps -A -o pid=,pgid= 2>/dev/null) || return 0
  own=$(ps -o pgid= -p "$$" 2>/dev/null | tr -d ' ')
  # one stream, each line tagged: T the tree, S the snapshot, P pid and group, G a group already followed
  out=$({ printf '%s\n' "$1" | sed 's/^/T /'; printf '%s\n' "$2" | sed 's/^/S /'; printf '%s\n' "$pg" | sed 's/^/P /'
          for g in $HEAVY_SLOT_GROUPS; do printf 'G %s\n' "$g"; done; } | awk -v own="$own" '
    $1 == "T" && $2 != "" { t[$2] = 1; line = $0; sub(/^T /, "", line); tree[++nt] = line; next }
    $1 == "S" && $2 != "" { line = $0; sub(/^S[ ]+[0-9]+[ ]+[0-9]+[ ]+/, "", line); snap[$2] = line; next }
    $1 == "P" && $2 != "" { grp[$2] = $3; next }
    $1 == "G" { g[$2] = 1 }
    END {
      for (p in t) if ((p in grp) && grp[p] != own && grp[p] != "") g[grp[p]] = 1
      printf "G"; for (k in g) printf " %s", k; printf "\n"
      for (i = 1; i <= nt; i++) print tree[i]
      for (p in grp) if ((grp[p] in g) && !(p in t) && (p in snap)) print p, snap[p]
    }')
  HEAVY_SLOT_GROUPS=$(printf '%s\n' "$out" | sed -n '1s/^G//p')
  HEAVY_SLOT_GROUP_TREE=$(printf '%s\n' "$out" | sed '1d')
}
# a kill of a tree that holds the recording take: record it now, its exit trap will not run
heavy_slot_record_kill() {
  [ -n "${HEAVY_SLOT_TAKE_ROOT:-}" ] || return 0
  if printf '%s\n' "$1" | awk -v r="$HEAVY_SLOT_TAKE_ROOT" '$1 == r { found = 1 } END { exit !found }'; then
    heavy_slot_record_take 137 "$2"
    rm -f "${HEAVY_SLOT_PEAK_FILE:-}" "${HEAVY_SLOT_PEAK_FILE:-}.note" 2>/dev/null || true
  elif [ -n "${HEAVY_SLOT_PEAK_FILE:-}" ]; then
    # the recording take outlives this kill and records at its exit: leave it the reason
    printf '%s\n' "$2" > "$HEAVY_SLOT_PEAK_FILE.note" 2>/dev/null || true
  fi
}
# the watcher's peak, kept as the largest any watcher of this take has seen (a nested watcher sees a smaller tree)
heavy_slot_note_peak() {
  local f=${HEAVY_SLOT_PEAK_FILE:-} old
  [ -n "$f" ] || return 0
  # a watcher still sampling after the take recorded would make the peak file again, and the lost-take recorder would
  # then write a second row for the same take (2026-11-14: takes.tsv had each take twice, the second "lost")
  [ -e "$f.done" ] && return 0
  old=$(cat "$f" 2>/dev/null) || old=0
  case "$old" in '' | *[!0-9]*) old=0 ;; esac
  if [ "$1" -gt "$old" ]; then printf '%s\n' "$1" > "$f" 2>/dev/null || true; fi
}
# memory: `heavy_slot_watch <pid> &` sums the RSS of that process and its descendants about once a second
# (ALICE_HEAVY_SLOT_RSS_POLL) and past ALICE_HEAVY_SLOT_RSS_CAP_MIB (default 8192 = 8 GiB) kills the tree (itself
# aside) and prints `[heavy-slot] RSS cap <N>GiB exceeded by <pid> <cmd> (tree <M> MiB)`. It also sums the trees of
# every held slot; past ALICE_HEAVY_SLOT_RSS_TOTAL_MIB (default 20480 = 20 GiB) the watcher whose slot holds the
# largest tree kills its own tree and prints `[heavy-slot] RSS total cap ...` (every slot's watcher looks, so no
# coordinator is needed). It returns once the process is gone. A time limit does not bound memory (a test that loops
# while pushing fills the machine inside its timeout), this does. A `ps` that cannot list the tree turns the cap off
# with a note, not silently.
heavy_slot_cap_ok() {
  case "${ALICE_HEAVY_SLOT_RSS_CAP_MIB:-8192}" in
    '' | *[!0-9]* | 0) echo "heavy slot: ALICE_HEAVY_SLOT_RSS_CAP_MIB must be a positive integer" >&2; return 2 ;;
  esac
  case "${ALICE_HEAVY_SLOT_RSS_TOTAL_MIB:-20480}" in
    '' | *[!0-9]* | 0) echo "heavy slot: ALICE_HEAVY_SLOT_RSS_TOTAL_MIB must be a positive integer" >&2; return 2 ;;
  esac
}
# from a `ps -A -o pid=,ppid=,rss=,comm=` listing on stdin: "<pid> <rss KiB> <command>" for the process $1 and its
# descendants, leaving out $2 and its descendants
heavy_slot_tree_in() {
  awk -v root="$1" -v skip="${2:-}" '
    { pid[NR] = $1; ppid[NR] = $2; rss[NR] = $3; $1 = $2 = $3 = ""; cmd[NR] = substr($0, 4) }
    END {
      in_tree[root] = 1; grew = 1
      while (grew) {
        grew = 0
        for (i = 1; i <= NR; i++)
          if (!(pid[i] in in_tree) && pid[i] != skip && (ppid[i] in in_tree)) { in_tree[pid[i]] = 1; grew = 1 }
      }
      for (i = 1; i <= NR; i++) if (pid[i] in in_tree) print pid[i], rss[i], cmd[i]
    }'
}
heavy_slot_tree() { ps -A -o pid=,ppid=,rss=,comm= | heavy_slot_tree_in "$@"; }
# every process as `<pid> <ppid> <KiB> <comm>`, the KiB being its physical footprint (macOS: proc_pid_rusage
# ri_phys_footprint; Linux: RssAnon + RssShmem + VmSwap) and its RSS where that cannot be read. RSS alone misses memory the
# kernel compressed: on 2026-10-28 a test filling 16 GiB with zeros showed 3 MiB of RSS until the machine stopped
# (the compressor full), and live processes here showed 36 MiB of RSS on a 1.8 GiB footprint. Without python3 it
# is the plain RSS listing (heavy_slot_watch notes that once).
heavy_slot_mem_snapshot() {
  local listing
  listing=$(ps -A -o pid=,ppid=,rss=,comm= 2>/dev/null) || return 1
  if command -v python3 >/dev/null 2>&1; then
    printf '%s\n' "$listing" | python3 -c '
import ctypes, ctypes.util, sys
fp = None
if sys.platform == "darwin":
    try:
        lib = ctypes.CDLL(ctypes.util.find_library("proc") or "/usr/lib/libproc.dylib")
        class RU(ctypes.Structure):
            _fields_ = [("uuid", ctypes.c_uint8 * 16)] + [("f%d" % i, ctypes.c_uint64) for i in range(18)]
        def fp(pid):
            r = RU()
            return r.f7 // 1024 if lib.proc_pid_rusage(pid, 2, ctypes.byref(r)) == 0 else None
    except OSError:
        fp = None
elif sys.platform.startswith("linux"):
    def fp(pid):
        try:
            kb = {}
            for line in open("/proc/%d/status" % pid):
                k, _, v = line.partition(":")
                if k in ("RssAnon", "RssShmem", "VmSwap"):
                    kb[k] = int(v.split()[0])
            # anonymous + shared memory, resident or swapped (zswap included): file pages left out, as the
            # macOS footprint leaves them out
            return sum(kb.values()) if "RssAnon" in kb else None
        except (OSError, ValueError):
            return None
for line in sys.stdin:
    p = line.split(None, 3)
    if len(p) < 3 or not p[0].isdigit():
        continue
    kib = fp(int(p[0])) if fp else None
    print(p[0], p[1], p[2] if kib is None else kib, p[3].rstrip("\n") if len(p) > 3 else "")
' && return 0
  fi
  printf '%s\n' "$listing"
}
# MiB of the process $2 and its descendants (leaving out $3 and its) in a heavy_slot_mem_snapshot listing on stdin
heavy_slot_tree_mib() {
  heavy_slot_tree_in "$1" "${2:-}" | awk '{ s += $2 } END { printf "%d", s / 1024 }'
}
# whether $1 runs (a zombie has ended: a watcher started by the process it watches outlives it until reaped)
heavy_slot_alive() {
  local st
  st=$(ps -o stat= -p "$1" 2>/dev/null) || return 1
  case "$st" in '' | *Z*) return 1 ;; esac
}
heavy_slot_watch() {
  local root=$1 cap="${ALICE_HEAVY_SLOT_RSS_CAP_MIB:-8192}" total_cap="${ALICE_HEAVY_SLOT_RSS_TOTAL_MIB:-20480}" \
    poll="${ALICE_HEAVY_SLOT_RSS_POLL:-1}" d self snap tree sum top f h hsum total largest largest_sum slots \
    largest_new largest_new_sum victim \
    peak=0 samples=0 noted=0
  d=$(heavy_slot_dir)
  self=$(exec sh -c 'echo "$PPID"')
  if kill -0 "$root" 2>/dev/null && ! heavy_slot_alive "$root"; then
    echo "[heavy-slot] note: ps does not show pid $root here, so the memory cap is off" >&2
    return 0
  fi
  if ! command -v python3 >/dev/null 2>&1; then
    echo "[heavy-slot] note: no python3 here, so the memory cap reads RSS (it does not see compressed memory)" >&2
  fi
  while heavy_slot_alive "$root"; do
    snap=$(heavy_slot_mem_snapshot 2>/dev/null) || snap=""
    tree=$(printf '%s\n' "$snap" | heavy_slot_tree_in "$root" "$self" 2>/dev/null) || tree=""
    if [ -z "$tree" ]; then
      if [ "$noted" = 0 ] && heavy_slot_alive "$root"; then
        echo "[heavy-slot] note: ps does not list pid $root here, so the memory cap is off" >&2
        noted=1
      fi
    else
      heavy_slot_group_tree "$tree" "$snap"
      tree=$HEAVY_SLOT_GROUP_TREE
      sum=$(printf '%s\n' "$tree" | awk '{ s += $2 } END { printf "%d", s / 1024 }')
      samples=$((samples + 1))
      if [ "$sum" -gt "$peak" ]; then peak=$sum; heavy_slot_note_peak "$peak"; fi
      if [ "$sum" -gt "$cap" ]; then
        top=$(printf '%s\n' "$tree" | sort -k2,2nr | head -1)
        heavy_slot_record_kill "$tree" rss-cap
        # shellcheck disable=SC2046 # one pid per word
        kill -9 $(printf '%s\n' "$tree" | cut -d" " -f1) 2>/dev/null || true
        echo "[heavy-slot] RSS cap $(awk -v c="$cap" 'BEGIN { printf "%g", c / 1024 }')GiB exceeded by ${top%% *} $(printf '%s\n' "$top" | cut -d" " -f3-) (tree $sum MiB, physical footprint)" >&2
        return 137
      fi
      # every held slot's tree (by its holder), the total, the holder of the largest, and the largest of the
      # slots that have a watcher: `info` without a number is the one slot of a copy of this block from before
      # the slots were numbered (an older generated preflight), which has no watcher, so the watcher of the
      # largest numbered slot stops it when it is the largest
      total=0 largest="" largest_sum=-1 largest_new="" largest_new_sum=-1 slots=0
      for f in $(heavy_slot_live "$d" | awk '{ print "keeper:" $2 }') "$d/info"; do
        case "$f" in keeper:*) h=${f#keeper:} ;; *) h=$(sed -n 1p "$f" 2>/dev/null) || continue ;; esac
        [ -n "$h" ] && kill -0 "$h" 2>/dev/null || continue
        hsum=$(printf '%s\n' "$snap" | heavy_slot_tree_mib "$h")
        total=$((total + hsum)) slots=$((slots + 1))
        if [ "$hsum" -gt "$largest_sum" ]; then largest=$h largest_sum=$hsum; fi
        if [ "$f" != "$d/info" ] && [ "$hsum" -gt "$largest_new_sum" ]; then largest_new=$h largest_new_sum=$hsum; fi
      done
      if [ "$total" -gt "$total_cap" ] && [ -n "$largest" ]; then
        victim=""
        if [ "$largest" = "${ALICE_HEAVY_SLOT_HELD:-}" ]; then
          victim=$tree
        elif [ "$largest" = "$(sed -n 1p "$d/info" 2>/dev/null)" ] && [ "$largest_new" = "${ALICE_HEAVY_SLOT_HELD:-}" ]; then
          victim=$(printf '%s\n' "$snap" | heavy_slot_tree_in "$largest")
        fi
        if [ -n "$victim" ]; then
          top=$(printf '%s\n' "$victim" | sort -k2,2nr | head -1)
          heavy_slot_record_kill "$victim" rss-total-cap
          # shellcheck disable=SC2046 # one pid per word
          kill -9 $(printf '%s\n' "$victim" | cut -d" " -f1) 2>/dev/null || true
          echo "[heavy-slot] RSS total cap $(awk -v c="$total_cap" 'BEGIN { printf "%g", c / 1024 }')GiB exceeded ($total MiB in $slots slots), killed the largest tree ${top%% *} $(printf '%s\n' "$top" | cut -d" " -f3-) ($largest_sum MiB)" >&2
          [ "$largest" = "${ALICE_HEAVY_SLOT_HELD:-}" ] && return 137
        fi
      fi
    fi
    sleep "$poll"
  done
  if [ "${ALICE_HEAVY_SLOT_RSS_REPORT:-}" = 1 ]; then echo "[heavy-slot] RSS peak $peak MiB over $samples samples" >&2; fi
  return 0
}
# --- heavy slot (end) ---
heavy_slot_acquire "$(basename "$PWD")" preflight || exit 1
gh_files=$(mktemp -d)
gh_local=0
[ -z "${GITHUB_ENV:-}" ] && gh_local=1
export GITHUB_OUTPUT="${GITHUB_OUTPUT:-$gh_files/output}" GITHUB_STEP_SUMMARY="${GITHUB_STEP_SUMMARY:-$gh_files/summary}"
export RUNNER_TEMP="${RUNNER_TEMP:-$gh_files/runner-temp}"
mkdir -p "$RUNNER_TEMP"
trap 'heavy_slot_release; rm -rf "$gh_files"' EXIT
# gh_step <job>: this job's $GITHUB_ENV / $GITHUB_PATH, with what its earlier steps wrote loaded
gh_step() {
  [ "$gh_local" = 1 ] || return 0
  export GITHUB_ENV="$gh_files/$1.env" GITHUB_PATH="$gh_files/$1.path"
  touch "$GITHUB_ENV" "$GITHUB_PATH"
  local line
  while IFS= read -r line || [ -n "$line" ]; do
    case "$line" in
      '') ;;
      *'<<'*) echo "preflight: a multi-line \$GITHUB_ENV value is not reproduced here: $line" >&2; return 1 ;;
      [A-Za-z_]*=*) export "${line%%=*}=${line#*=}" ;;
      *) echo "preflight: not a NAME=value line in \$GITHUB_ENV: $line" >&2; return 1 ;;
    esac
  done < "$GITHUB_ENV"
  while IFS= read -r line || [ -n "$line" ]; do [ -n "$line" ] && PATH="$line:$PATH"; done < "$GITHUB_PATH"
  export PATH
}
heavy_slot_cap_ok || exit 2
heavy_slot_watch "$$" >/dev/null &

quick=0
[[ "${1:-}" == "--quick" ]] && quick=1

step() { printf '\n\033[1;34m== %s\033[0m\n' "$*"; }
# `cargo clippy` reuses fresh `cargo check` artifacts and then lints nothing;
# touching the crate roots invalidates only this repo's fingerprints.
relint() { git ls-files | grep -E '(^|/)src/(lib|main)\.rs$' | xargs -r touch; }
need() { command -v "$1" >/dev/null 2>&1 || { echo "missing tool: $1 ($2)" >&2; exit 1; }; }
has_toolchain() { grep -q "^$1" <<<"$(rustup toolchain list)"; }

# Steps CI runs that this file cannot reproduce locally (they can only fail remotely):
#   - ci.yml:web:Install wasm-bindgen-cli (needs network / runner-only)
#   - ci.yml:web:Stall rule (unit tests) (no cargo / grep)
#   - ci.yml:web: uses actions/setup-node@v5 (an action this file cannot run)
#   - ci.yml:web:Browser checks (SwiftShader WebGPU, then none) (no cargo / grep)
#   - ci.yml:web:Install Mesa software Vulkan (lavapipe) (needs network / runner-only)
#   - ci.yml:web:Browser checks on software WebGPU over Mesa Vulkan (20 runs) (needs network / runner-only)
#   - ci.yml:web:Stall gate catches a held render thread (4 runs, PROBE_HOLD_MS=2000) (needs network / runner-only)
#   - security-audit.yml:changes:decide (reads AFTER, BEFORE, CREATED, EVENT, FORCED, PR_BASE, PR_HEAD, a value only the runner has)
#   - security-audit.yml:coverage:Run coverage (needs network / runner-only)
#   - security-audit.yml:semver-checks:Run cargo semver-checks (needs network / runner-only)
#   - security-audit.yml:gate:gate (reads NEEDS, a value only the runner has)
#   - fuzz.yml:changes:decide (reads AFTER, BEFORE, CREATED, EVENT, FORCED, PR_BASE, PR_HEAD, a value only the runner has)
#   - fuzz.yml:fuzz:Install Linux dependencies (for wgpu / winit build) [target=fuzz_alice_decode] (no cargo / grep)
#   - fuzz.yml:fuzz:Install cargo-fuzz [target=fuzz_alice_decode] (a tool install, setup)
#   - fuzz.yml:fuzz:Set fuzz duration [target=fuzz_alice_decode] (no cargo / grep)
#   - fuzz.yml:fuzz:Run fuzz target (time-boxed) [target=fuzz_alice_decode] (runner-only (open-ended or unprovable-bounded fuzz run))
#   - fuzz.yml:fuzz:Install Linux dependencies (for wgpu / winit build) [target=fuzz_alice_header] (no cargo / grep)
#   - fuzz.yml:fuzz:Install cargo-fuzz [target=fuzz_alice_header] (a tool install, setup)
#   - fuzz.yml:fuzz:Set fuzz duration [target=fuzz_alice_header] (no cargo / grep)
#   - fuzz.yml:fuzz:Run fuzz target (time-boxed) [target=fuzz_alice_header] (runner-only (open-ended or unprovable-bounded fuzz run))
#   - fuzz.yml:fuzz:Install Linux dependencies (for wgpu / winit build) [target=fuzz_alice_metadata] (no cargo / grep)
#   - fuzz.yml:fuzz:Install cargo-fuzz [target=fuzz_alice_metadata] (a tool install, setup)
#   - fuzz.yml:fuzz:Set fuzz duration [target=fuzz_alice_metadata] (no cargo / grep)
#   - fuzz.yml:fuzz:Run fuzz target (time-boxed) [target=fuzz_alice_metadata] (runner-only (open-ended or unprovable-bounded fuzz run))
#   - fuzz.yml:gate:gate (reads NEEDS, a value only the runner has)
#   - public-text.yml:public-text: uses actions/setup-python@5fda3b95a4ea91299a34e894583c3862153e4b97 (an action this file cannot run)
#   - public-text.yml:public-text:Public text (tree at HEAD, then the pushed commits) (no cargo / grep)

need actionlint "brew install actionlint"
need cargo-audit "cargo install cargo-audit --locked"
need cargo-deny "cargo install cargo-deny --locked"
has_toolchain 1.92.0 || { echo "missing toolchain 1.92.0 (rustup toolchain install 1.92.0)" >&2; exit 1; }
command -v python >/dev/null 2>&1 || python() { python3 "$@"; }

step "ci.yml / test: Build (default)"
(
  gh_step ci.yml.test
( export CARGO_TERM_COLOR="always"; cargo build --lib )
)

step "ci.yml / test: Check the analytics bridge (optional feature)"
(
  gh_step ci.yml.test
( export CARGO_TERM_COLOR="always"; cargo check --lib --features analytics )
)

step "ci.yml / fmt: Check formatting"
(
  gh_step ci.yml.fmt
( export CARGO_TERM_COLOR="always"; cargo fmt -- --check )
)

step "ci.yml / fmt: no-direct-rkyv guard (tests)"
(
  gh_step ci.yml.fmt
( export CARGO_TERM_COLOR="always"; python3 scripts/test_no_direct_rkyv.py )
)

step "ci.yml / fmt: no-direct-rkyv guard"
(
  gh_step ci.yml.fmt
( export CARGO_TERM_COLOR="always"; python3 scripts/no_direct_rkyv.py )
)

step "ci.yml / web: Format"
(
  gh_step ci.yml.web
(cd "crates/alice-view-web" && ( export CARGO_TERM_COLOR="always"; cargo +1.92.0 fmt -- --check ))
)

step "ci.yml / web: Clippy (wasm32, with and without the probe)"
(
  gh_step ci.yml.web
relint
grep -q '^wasm32-unknown-unknown$' <<<"$(rustup target list --installed --toolchain 1.92.0)" || rustup target add wasm32-unknown-unknown --toolchain 1.92.0
grep -q '^wasm32-unknown-unknown$' <<<"$(rustup target list --installed --toolchain 1.92.0)" || rustup target add wasm32-unknown-unknown --toolchain 1.92.0
(cd "crates/alice-view-web"
(
  export CARGO_TERM_COLOR="always"
  cargo +1.92.0 clippy --release --target wasm32-unknown-unknown -- -D warnings
  cargo +1.92.0 clippy --release --target wasm32-unknown-unknown --features probe -- -D warnings
)
)
)

step "ci.yml / web: wasm-bindgen CLI version check (unit tests)"
(
  gh_step ci.yml.web
(cd "crates/alice-view-web" && ( export CARGO_TERM_COLOR="always"; python3 probe/test_wasm_bindgen_match.py ))
)

step "ci.yml / web: Size gate"
(
  gh_step ci.yml.web
(cd "crates/alice-view-web" && ( export CARGO_TERM_COLOR="always"; python3 probe/size.py ))
)

step "ci.yml / actionlint: actionlint"
(
  gh_step ci.yml.actionlint
actionlint .github/workflows/*.yml
)

step "security-audit.yml / deny: Run cargo deny check all"
(
  gh_step security-audit.yml.deny
( export CARGO_TERM_COLOR="always" CARGO_NET_RETRY="5" CARGO_HTTP_MULTIPLEXING="false"; cargo deny --all-features check all )
)

step "security-audit.yml / unused-deps: run"
(
  gh_step security-audit.yml.unused-deps
( export CARGO_TERM_COLOR="always" CARGO_NET_RETRY="5" CARGO_HTTP_MULTIPLEXING="false"; cargo-machete )
)

step "security-audit.yml / stub-guard: Detect todo! / unimplemented! / panic!(STUB) in src/**"
(
  gh_step security-audit.yml.stub-guard
(
  export CARGO_TERM_COLOR="always" CARGO_NET_RETRY="5" CARGO_HTTP_MULTIPLEXING="false"
  set -eo pipefail
  # STUB (uppercase) のみに絞る (小文字の stub は通常語として現れる)
  # trait default method の `panic!("... not implemented by this backend")`
  # 等は legitimate な pattern で stub ではない
  hits=$(grep -rnE 'todo!\(|unimplemented!\(|panic!\([^)]*STUB' \
    src/ --include="*.rs" \
    --exclude-dir=bin \
    || true)
  if [ -n "$hits" ]; then
    echo "❌ Stub / unimplemented / STUB panic detected in src/ (production path):"
    echo "$hits"
    echo ""
    echo "Fix: use ? operator, Result<T, E>, or remove the stub."
    exit 1
  fi
  echo "✓ No todo! / unimplemented! / panic!(STUB) in src/ (excluding src/bin/)"
)
)

step "security-audit.yml / stub-guard: Detect dbg!() residual in src/**"
(
  gh_step security-audit.yml.stub-guard
(
  export CARGO_TERM_COLOR="always" CARGO_NET_RETRY="5" CARGO_HTTP_MULTIPLEXING="false"
  set -eo pipefail
  hits=$(grep -rn 'dbg!(' src/ --include="*.rs" || true)
  if [ -n "$hits" ]; then
    echo "❌ dbg!() macro left in src/:"
    echo "$hits"
    exit 1
  fi
  echo "✓ No dbg!() in src/"
)
)

step "security-audit.yml / stub-guard: Detect TODO / FIXME / XXX / HACK (informational)"
(
  gh_step security-audit.yml.stub-guard
(
  export CARGO_TERM_COLOR="always" CARGO_NET_RETRY="5" CARGO_HTTP_MULTIPLEXING="false"
  set -eo pipefail
  hits=$(grep -rnE 'TODO|FIXME|XXX|HACK' src/ --include="*.rs" || true)
  if [ -n "$hits" ]; then
    echo "::warning::TODO/FIXME/XXX/HACK found in src/ (informational, not blocking):"
    head -50 <<<"$hits"
  else
    echo "✓ No TODO/FIXME/XXX/HACK in src/"
  fi
)
)

step "security-audit.yml / semver-checks: Self-test: semver baseline and decision"
(
  gh_step security-audit.yml.semver-checks
( export CARGO_TERM_COLOR="always" CARGO_NET_RETRY="5" CARGO_HTTP_MULTIPLEXING="false"; python3 .github/test_semver_gate.py )
)

step "public-text.yml / public-text: Tests of scripts/public_text.sh (no network)"
(
  gh_step public-text.yml.public-text
( python scripts/test_public_text.py )
)

step "fuzz.yml / build every fuzz target (nightly; the replay needs the runner)"
if has_toolchain nightly && cargo +nightly fuzz --version >/dev/null 2>&1; then
  (cd fuzz && cargo +nightly fuzz build)
else
  echo 'PREFLIGHT-SKIP: fuzz.yml / build every fuzz target: nightly / cargo-fuzz not installed' >&2
fi

if [[ $quick -eq 1 ]]; then
  echo; echo "preflight --quick OK (test / bench suites skipped)"; exit 0
fi

step "ci.yml / test: Test"
(
  gh_step ci.yml.test
( export CARGO_TERM_COLOR="always"; cargo test --lib )
)

step "ci.yml / test: Test (all features)"
(
  gh_step ci.yml.test
( export CARGO_TERM_COLOR="always"; cargo test --lib --all-features )
)

step "security-audit.yml / audit: Run cargo audit"
(
  gh_step security-audit.yml.audit
(
  export CARGO_TERM_COLOR="always" CARGO_NET_RETRY="5" CARGO_HTTP_MULTIPLEXING="false"
  cargo audit --deny yanked \
    --ignore RUSTSEC-2026-0235
)
)

# CI runs that this run does not: named and counted in the receipt (run-preflight)
not_run() { echo "PREFLIGHT-NOT-RUN: $*"; }
# an OS leg runs here only when the host is of its family (ubuntu on Linux, macos on macOS)
leg_not_run() {
  case "$2:$(uname -s)" in
    ubuntu*:Linux|macos*:Darwin) ;;
    *) not_run "$1 [runs-on=$2] (an OS leg this host does not run)" ;;
  esac
}
not_run 'ci.yml:web:Stall rule (unit tests) (no cargo / grep)'
not_run 'ci.yml:web: uses actions/setup-node@v5 (an action this file cannot run)'
not_run 'ci.yml:web:Browser checks (SwiftShader WebGPU, then none) (no cargo / grep)'
not_run 'ci.yml:web:Browser checks on software WebGPU over Mesa Vulkan (20 runs) (needs network / runner-only)'
not_run 'ci.yml:web:Stall gate catches a held render thread (4 runs, PROBE_HOLD_MS=2000) (needs network / runner-only)'
not_run 'security-audit.yml:coverage:Run coverage (needs network / runner-only)'
not_run 'security-audit.yml:semver-checks:Run cargo semver-checks (needs network / runner-only)'
not_run 'fuzz.yml:fuzz:Run fuzz target (time-boxed) [target=fuzz_alice_decode] (runner-only (open-ended or unprovable-bounded fuzz run))'
not_run 'fuzz.yml:fuzz:Run fuzz target (time-boxed) [target=fuzz_alice_header] (runner-only (open-ended or unprovable-bounded fuzz run))'
not_run 'fuzz.yml:fuzz:Run fuzz target (time-boxed) [target=fuzz_alice_metadata] (runner-only (open-ended or unprovable-bounded fuzz run))'
not_run 'public-text.yml:public-text: uses actions/setup-python@5fda3b95a4ea91299a34e894583c3862153e4b97 (an action this file cannot run)'
not_run 'public-text.yml:public-text:Public text (tree at HEAD, then the pushed commits) (no cargo / grep)'
leg_not_run ci.yml:test macos-latest
leg_not_run ci.yml:test ubuntu-latest

echo; echo "preflight OK"
