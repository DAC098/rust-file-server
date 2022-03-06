const http = require("http");
const JSONBig = require("json-bigint");

const JSONLocal = JSONBig({strict: true, useNativeBigInt: true});

/**
 * 
 * @param {http.IncomingMessage} req 
 * @returns Promise
 */
async function readBody(req) {
    return new Promise((resolve, reject) => {
        let chunk = "";

        req.on("data", data => {
            chunk += data.toString("utf8");
        });

        req.on("end", () => {
            console.log("end");
            resolve(chunk);
        });
    })
}

let server = http.createServer(async (req, res) => {
    console.log(req.method, req.url);

    let body = await readBody(req);
    let json = JSONLocal.parse(body);

    console.log(json);

    res.writeHead(204);
    res.end();
});

server.listen(8888, "0.0.0.0", () => {
    console.log("server listening on 0.0.0.0:8888");
});
server.listen(8888, "::", () => {
    console.log("server listening on :::8888")
});