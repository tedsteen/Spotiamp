/** @type {import('./$types').PageLoad} */
export async function load({ }) {
    await new Promise((resolve) => {
        resolve(undefined); //TODO: wait for playlist to be ready?
    });

    return {};
}