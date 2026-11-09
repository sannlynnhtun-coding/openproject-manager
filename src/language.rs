pub fn text(mm: bool, english: &str) -> &str {
    if !mm {
        return english;
    }
    match english {
        "Light mode" => "အလင်းစနစ်",
        "Dark mode" => "အမှောင်စနစ်",
        "Windows service:" => "Windows service:",
        "Web: Checking" => "Web: စစ်ဆေးနေသည်",
        "Web: Ready" => "Web: အသင့်ဖြစ်ပါပြီ",
        "Web: Starting" => "Web: စတင်နေသည်",
        "Web: Unavailable" => "Web: မရရှိသေးပါ — မှတ်တမ်း စစ်ဆေးပါ",
        "Web: Not ready" => "Web: အသင့်မဖြစ်သေးပါ",
        "Web: Not running" => "Web: အလုပ်မလုပ်သေးပါ",
        "Web: Database migration required" => "Web: Database migration လိုအပ်သည်",
        "Service is already stopped." => "Service ရပ်ထားပြီး ဖြစ်ပါသည်။",
        "Service start requested. OpenProject web server may still be initializing." => {
            "Service စတင်ရန် တောင်းဆိုပြီးပါပြီ။ OpenProject web server ပြင်ဆင်နေဆဲ ဖြစ်နိုင်ပါသည်။"
        }
        "Service stop requested." => "Service ရပ်ရန် တောင်းဆိုပြီးပါပြီ။",
        "Service removed." => "Service ဖယ်ရှားပြီးပါပြီ။",
        "Use existing service" => "ရှိပြီးသား Service ချိတ်ဆက်ရန်",
        "Existing service connected. Use Start / Stop / Restart. Configuration was not changed." => {
            "ရှိပြီးသား Service ချိတ်ဆက်ပြီးပါပြီ။ စတင် / ရပ် / ပြန်စတင် ကို အသုံးပြုနိုင်ပါပြီ။ ဆက်တင်များကို မပြောင်းထားပါ။"
        }
        "Show manager" => "Manager ပြန်ဖွင့်ရန်",
        "Exit" => "App မှ ထွက်ရန်",
        "Stop before editing settings. Close (×) hides the manager in the system tray. Right-click the tray icon for controls or Exit." => {
            "ဆက်တင်မပြောင်းမီ Service ကို ရပ်ပါ။ × နှိပ်လျှင် Manager သည် System tray ထဲတွင် ရှိနေမည်။ Tray icon ကို ညာကလစ်နှိပ်၍ Service ထိန်းချုပ်ရန် သို့မဟုတ် App မှ ထွက်ရန် ရွေးနိုင်ပါသည်။"
        }
        "5. Close this window to keep the manager in the system tray. Double-click its tray icon to reopen it. Right-click for Start, Stop, Restart and Exit. Exit closes only the manager; the service keeps running." => {
            "၅။ ဝင်းဒိုးပိတ်လျှင် Manager သည် System tray ထဲတွင် ရှိနေမည်။ Tray icon ကို နှစ်ချက်နှိပ်၍ ပြန်ဖွင့်ပါ။ ညာကလစ်နှိပ်၍ စတင်၊ ရပ်၊ ပြန်စတင် သို့မဟုတ် App မှ ထွက်နိုင်ပါသည်။ App မှ ထွက်သော်လည်း OpenProject service ဆက်အလုပ်လုပ်နေပါမည်။"
        }
        "Setup instructions" => "အသုံးပြုနည်း အဆင့်ဆင့်",
        "WINDOWS SERVICE · Start automatically when Windows starts" => {
            "WINDOWS SERVICE · Windows တက်လာလျှင် အလိုအလျောက် စတင်မည်"
        }
        "Refresh status" => "အခြေအနေ ပြန်စစ်ရန်",
        "OpenProject installation folder" => "OpenProject တည်ရှိရာ ဖိုလ်ဒါ",
        "Browse…" => "ဖိုလ်ဒါ ရွေးရန်…",
        "Host IP" => "Host IP လိပ်စာ",
        "Port" => "Port နံပါတ်",
        "Save settings" => "ဆက်တင် သိမ်းရန်",
        "Install service" => "Service တပ်ဆင်ရန်",
        "Start" => "စတင်ရန်",
        "Stop" => "ရပ်ရန်",
        "Restart" => "ပြန်စတင်ရန်",
        "Remove service" => "Service ဖယ်ရှားရန်",
        "Open OpenProject" => "OpenProject ဖွင့်ရန်",
        "View log" => "မှတ်တမ်း ကြည့်ရန်",
        "Login details" => "အကောင့်ဝင်ရန် အချက်အလက်",
        "Partially running" => "Service အချို့သာ အလုပ်လုပ်နေသည်",
        "Incomplete" => "Service များ မပြည့်စုံသေးပါ",
        "Running" => "အလုပ်လုပ်နေသည်",
        "Stopped" => "ရပ်ထားသည်",
        "Starting" => "စတင်နေသည်",
        "Stopping" => "ရပ်တန့်နေသည်",
        "Not installed" => "မတပ်ဆင်ရသေးပါ",
        "Press Refresh to check service" => "Service အခြေအနေ စစ်ရန် ပြန်စစ်ရန် ခလုတ်ကို နှိပ်ပါ",
        "Wait for Web: Ready, then click Open OpenProject. Use Start if the service is stopped." => {
            "Web အဆင်သင့်ဖြစ်သည်အထိ စောင့်ပြီး OpenProject ဖွင့်ရန် ကိုနှိပ်ပါ။ Service ရပ်ထားလျှင် စတင်ရန် ကိုနှိပ်ပါ။"
        }
        "Settings saved. Start the service to apply them." => {
            "ဆက်တင် သိမ်းပြီးပါပြီ။ အသုံးပြုရန် Service ကို စတင်ပါ။"
        }
        "Opened OpenProject in your browser." => "Browser တွင် OpenProject ဖွင့်ပြီးပါပြီ။",
        "Background operation ended unexpectedly" => "နောက်ခံလုပ်ဆောင်မှု မမျှော်လင့်ဘဲ ရပ်သွားပါသည်",
        "127.0.0.1 = this computer only · 0.0.0.0 = all IPv4 interfaces. LAN access needs your firewall configuration." => {
            "127.0.0.1 = ဤစက်တွင်သာ သုံးရန် · 0.0.0.0 = IPv4 ကွန်ရက်အားလုံး။ LAN မှ ဝင်သုံးရန် Firewall ကို သတ်မှတ်ပေးပါ။"
        }
        "Stop before editing settings. Install service also starts OpenProject. Closing this window leaves the service running." => {
            "ဆက်တင်မပြောင်းမီ Service ကို ရပ်ပါ။ Service တပ်ဆင်လျှင် OpenProject ကိုပါ စတင်ပေးပါမည်။ ဤဝင်းဒိုးပိတ်သော်လည်း Service ဆက်အလုပ်လုပ်နေပါမည်။"
        }
        "1. Run OpenProjectManager-Setup.exe and approve the administrator prompt. Ruby, PostgreSQL and OpenProject are included. Setup creates a private database and three Windows services. Existing data is kept." => {
            "၁။ OpenProjectManager-Setup.exe ကိုဖွင့်ပြီး Administrator ခွင့်ပြုပါ။ Ruby၊ PostgreSQL နှင့် OpenProject ပါဝင်ပါသည်။ ကိုယ်ပိုင် Database နှင့် Windows Service သုံးခုကို တပ်ဆင်ပေးပြီး ရှိပြီးသား Data ကို ထိန်းသိမ်းပေးပါမည်။"
        }
        "2. Wait for Web: Ready, then click Open OpenProject. First startup can take several minutes. Use View log if startup fails, or Start if the service is stopped." => {
            "၂။ Web အဆင်သင့်ဖြစ်သည်အထိ စောင့်ပြီး OpenProject ဖွင့်ရန် ကိုနှိပ်ပါ။ ပထမဆုံးအကြိမ် မိနစ်အနည်းငယ် ကြာနိုင်ပါသည်။ မစတင်နိုင်လျှင် မှတ်တမ်း ကြည့်ရန်၊ ရပ်ထားလျှင် စတင်ရန် ကိုနှိပ်ပါ။"
        }
        "3. Click Login details for your generated admin password. The default address is http://127.0.0.1:8088." => {
            "၃။ ဖန်တီးပေးထားသော admin Password ကို အကောင့်ဝင်ရန် အချက်အလက် ခလုတ်မှ ကြည့်ပါ။ မူလလိပ်စာမှာ http://127.0.0.1:8088 ဖြစ်ပါသည်။"
        }
        "4. Keep database, attachments and private credentials backed up. Reinstall repairs this bundle; it does not migrate the earlier prototype installation." => {
            "၄။ Database၊ တွဲတင်ဖိုင်များနှင့် လျှို့ဝှက်အချက်အလက်များကို Backup သိမ်းပါ။ ပြန်တပ်ဆင်ခြင်းသည် ဤ Bundle ကိုသာ ပြုပြင်ပေးပြီး ယခင် Prototype Data ကို မရွှေ့ပြောင်းပေးပါ။"
        }
        "5. You can close the manager. The service keeps running and starts automatically next time Windows starts." => {
            "၅။ App ကို ပိတ်နိုင်ပါပြီ။ Service ဆက်အလုပ်လုပ်နေမည်ဖြစ်ပြီး Windows ပြန်တက်လာလျှင်လည်း အလိုအလျောက် စတင်ပါမည်။"
        }
        "To change settings later: Stop the service, save your changes, then Start. To change installation folders, remove the stopped service first." => {
            "နောက်ပိုင်း ဆက်တင်ပြောင်းရန် Service ကို ရပ်၊ ဆက်တင်သိမ်းပြီး ပြန်စတင်ပါ။ ဖိုလ်ဒါပြောင်းရန် ရပ်ထားသော Service ကို အရင်ဖယ်ရှားပါ။"
        }
        _ => english,
    }
}
