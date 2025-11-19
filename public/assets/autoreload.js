var autoreload_key = null;

async function autoreload_get_key() {
    const url = "/autoreload";
    try {
        const response = await fetch(url);
        if (!response.ok) {
            return null;
        }

        return await response.text();
    } catch (error) {
        console.error(error.message);
    }
}

async function autoreload_check() {
    const new_key = await autoreload_get_key();
    if (new_key === null) {
        return;
    }

    if (autoreload_key === null) {
        autoreload_key = new_key;
        return;
    }

    if (new_key !== autoreload_key) {
        autoreload_perform_reload();
    }
}

async function autoreload_perform_reload() {
    try {
        while (true) {
            const response = await fetch(window.location.href);
            if (!response.ok) {

            }

            autoreload_wait(100);
            break;
        }

        location.reload();
    } catch (error) {
        console.error(error.message);
    }
}

function autoreload_wait(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}

setInterval(autoreload_check, 1000);
