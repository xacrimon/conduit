var autoreload_es = null;
var triggered_reload = false;

async function autoreload_check() {
    if (autoreload_es == null) {
        autoreload_es = new EventSource("/autoreload");
        autoreload_es.onerror = function(_event) {
            if (!triggered_reload) {
                triggered_reload = true;
                autoreload_perform_reload();
            }
        };
    }
}

async function autoreload_perform_reload() {
    while (true) {
        await autoreload_wait(100);

        try {
            const response = await fetch(window.location.href);
            if (!response.ok) {
                console.error("response not ok")
                continue;
            }

            location.reload();
        } catch (error) {
            console.error(error.message);
        }
    }
}

function autoreload_wait(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}

setInterval(autoreload_check, 1000);
