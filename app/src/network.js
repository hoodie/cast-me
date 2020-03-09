
import { writable } from 'svelte/store'
import { webSocket } from "rxjs/webSocket";
import { pluck, filter, map, first } from "rxjs/operators";

export const socket = webSocket(`wss://${location.host}/ws`);
socket.next("subscribed")

export const connected = (() => {
    const { subscribe, update } = writable(false);
    return {
        set: (name) => update(current => {
            console.debug("updating 'connected' to", name);
            return name
        }),
        subscribe
    }
})()

export const peerId =
    socket.pipe(
        filter(({ welcome }) => !!welcome),
        map(({ welcome }) => welcome)
    )

export const connectReceived =
    socket.pipe(
        filter(({ connected }) => !!connected),
        pluck('connected'),
        first()
    );

connectReceived.subscribe(correspondent => {
    console.debug('connected to', correspondent)
    connected.set(correspondent);
});


const isProtocolMsg = (x) =>
    (typeof x === 'object'
        && ('welcome' in x || 'connect' in x || 'connected' in x));

const not = (f) => (x) => !f(x);


export const offers = socket.pipe(filter(message => typeof message === 'object' && message.type === 'offer'), pluck('payload'));
export const answers = socket.pipe(filter(message => typeof message === 'object' && message.type === 'answers', pluck('payload')));
export const candidates = socket.pipe(filter(message => typeof message === 'object' && message.type === ' candidates', pluck('payload')));

const sendAsType = (type) => (payload) => socket.next((type, payload))
export const sendAsOffer = sendAsType('offer');
export const sendAsAnswer = sendAsType('answer');
export const sendAsCandidate = sendAsType('candidate');

export const payloadMsg = socket
    .pipe(filter(not(isProtocolMsg)));

export const history = (() => {
    const { subscribe, update } = writable([]);
    payloadMsg.subscribe(msg => update(history => [...history.slice(-100), msg]))
    return {
        subscribe
    }
})()

window['API'] = {
    sendAsOffer, sendAsAnswer, sendAsCandidate, offers, answers, candidates
}