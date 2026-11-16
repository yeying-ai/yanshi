(function(){const t=document.createElement("link").relList;if(t&&t.supports&&t.supports("modulepreload"))return;for(const o of document.querySelectorAll('link[rel="modulepreload"]'))a(o);new MutationObserver(o=>{for(const l of o)if(l.type==="childList")for(const c of l.addedNodes)c.tagName==="LINK"&&c.rel==="modulepreload"&&a(c)}).observe(document,{childList:!0,subtree:!0});function s(o){const l={};return o.integrity&&(l.integrity=o.integrity),o.referrerPolicy&&(l.referrerPolicy=o.referrerPolicy),o.crossOrigin==="use-credentials"?l.credentials="include":o.crossOrigin==="anonymous"?l.credentials="omit":l.credentials="same-origin",l}function a(o){if(o.ep)return;o.ep=!0;const l=s(o);fetch(o.href,l)}})();function qx(e){return e&&e.__esModule&&Object.prototype.hasOwnProperty.call(e,"default")?e.default:e}var rm={exports:{}},Sl={};var sv;function AN(){if(sv)return Sl;sv=1;var e=Symbol.for("react.transitional.element"),t=Symbol.for("react.fragment");function s(a,o,l){var c=null;if(l!==void 0&&(c=""+l),o.key!==void 0&&(c=""+o.key),"key"in o){l={};for(var f in o)f!=="key"&&(l[f]=o[f])}else l=o;return o=l.ref,{$$typeof:e,type:a,key:c,ref:o!==void 0?o:null,props:l}}return Sl.Fragment=t,Sl.jsx=s,Sl.jsxs=s,Sl}var iv;function ON(){return iv||(iv=1,rm.exports=AN()),rm.exports}var d=ON(),lm={exports:{}},He={};var av;function MN(){if(av)return He;av=1;var e=Symbol.for("react.transitional.element"),t=Symbol.for("react.portal"),s=Symbol.for("react.fragment"),a=Symbol.for("react.strict_mode"),o=Symbol.for("react.profiler"),l=Symbol.for("react.consumer"),c=Symbol.for("react.context"),f=Symbol.for("react.forward_ref"),m=Symbol.for("react.suspense"),p=Symbol.for("react.memo"),_=Symbol.for("react.lazy"),y=Symbol.for("react.activity"),b=Symbol.iterator;function v(z){return z===null||typeof z!="object"?null:(z=b&&z[b]||z["@@iterator"],typeof z=="function"?z:null)}var S={isMounted:function(){return!1},enqueueForceUpdate:function(){},enqueueReplaceState:function(){},enqueueSetState:function(){}},w=Object.assign,k={};function j(z,Y,ce){this.props=z,this.context=Y,this.refs=k,this.updater=ce||S}j.prototype.isReactComponent={},j.prototype.setState=function(z,Y){if(typeof z!="object"&&typeof z!="function"&&z!=null)throw Error("takes an object of state variables to update or a function which returns an object of state variables.");this.updater.enqueueSetState(this,z,Y,"setState")},j.prototype.forceUpdate=function(z){this.updater.enqueueForceUpdate(this,z,"forceUpdate")};function A(){}A.prototype=j.prototype;function N(z,Y,ce){this.props=z,this.context=Y,this.refs=k,this.updater=ce||S}var T=N.prototype=new A;T.constructor=N,w(T,j.prototype),T.isPureReactComponent=!0;var E=Array.isArray;function O(){}var R={H:null,A:null,T:null,S:null},q=Object.prototype.hasOwnProperty;function Z(z,Y,ce){var de=ce.ref;return{$$typeof:e,type:z,key:Y,ref:de!==void 0?de:null,props:ce}}function oe(z,Y){return Z(z.type,Y,z.props)}function Q(z){return typeof z=="object"&&z!==null&&z.$$typeof===e}function ee(z){var Y={"=":"=0",":":"=2"};return"$"+z.replace(/[=:]/g,function(ce){return Y[ce]})}var X=/\/+/g;function L(z,Y){return typeof z=="object"&&z!==null&&z.key!=null?ee(""+z.key):Y.toString(36)}function I(z){switch(z.status){case"fulfilled":return z.value;case"rejected":throw z.reason;default:switch(typeof z.status=="string"?z.then(O,O):(z.status="pending",z.then(function(Y){z.status==="pending"&&(z.status="fulfilled",z.value=Y)},function(Y){z.status==="pending"&&(z.status="rejected",z.reason=Y)})),z.status){case"fulfilled":return z.value;case"rejected":throw z.reason}}throw z}function D(z,Y,ce,de,_e){var F=typeof z;(F==="undefined"||F==="boolean")&&(z=null);var G=!1;if(z===null)G=!0;else switch(F){case"bigint":case"string":case"number":G=!0;break;case"object":switch(z.$$typeof){case e:case t:G=!0;break;case _:return G=z._init,D(G(z._payload),Y,ce,de,_e)}}if(G)return _e=_e(z),G=de===""?"."+L(z,0):de,E(_e)?(ce="",G!=null&&(ce=G.replace(X,"$&/")+"/"),D(_e,Y,ce,"",function(Se){return Se})):_e!=null&&(Q(_e)&&(_e=oe(_e,ce+(_e.key==null||z&&z.key===_e.key?"":(""+_e.key).replace(X,"$&/")+"/")+G)),Y.push(_e)),1;G=0;var P=de===""?".":de+":";if(E(z))for(var ae=0;ae<z.length;ae++)de=z[ae],F=P+L(de,ae),G+=D(de,Y,ce,F,_e);else if(ae=v(z),typeof ae=="function")for(z=ae.call(z),ae=0;!(de=z.next()).done;)de=de.value,F=P+L(de,ae++),G+=D(de,Y,ce,F,_e);else if(F==="object"){if(typeof z.then=="function")return D(I(z),Y,ce,de,_e);throw Y=String(z),Error("Objects are not valid as a React child (found: "+(Y==="[object Object]"?"object with keys {"+Object.keys(z).join(", ")+"}":Y)+"). If you meant to render a collection of children, use an array instead.")}return G}function U(z,Y,ce){if(z==null)return z;var de=[],_e=0;return D(z,de,"","",function(F){return Y.call(ce,F,_e++)}),de}function B(z){if(z._status===-1){var Y=z._result;Y=Y(),Y.then(function(ce){(z._status===0||z._status===-1)&&(z._status=1,z._result=ce)},function(ce){(z._status===0||z._status===-1)&&(z._status=2,z._result=ce)}),z._status===-1&&(z._status=0,z._result=Y)}if(z._status===1)return z._result.default;throw z._result}var V=typeof reportError=="function"?reportError:function(z){if(typeof window=="object"&&typeof window.ErrorEvent=="function"){var Y=new window.ErrorEvent("error",{bubbles:!0,cancelable:!0,message:typeof z=="object"&&z!==null&&typeof z.message=="string"?String(z.message):String(z),error:z});if(!window.dispatchEvent(Y))return}else if(typeof process=="object"&&typeof process.emit=="function"){process.emit("uncaughtException",z);return}console.error(z)},te={map:U,forEach:function(z,Y,ce){U(z,function(){Y.apply(this,arguments)},ce)},count:function(z){var Y=0;return U(z,function(){Y++}),Y},toArray:function(z){return U(z,function(Y){return Y})||[]},only:function(z){if(!Q(z))throw Error("React.Children.only expected to receive a single React element child.");return z}};return He.Activity=y,He.Children=te,He.Component=j,He.Fragment=s,He.Profiler=o,He.PureComponent=N,He.StrictMode=a,He.Suspense=m,He.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE=R,He.__COMPILER_RUNTIME={__proto__:null,c:function(z){return R.H.useMemoCache(z)}},He.cache=function(z){return function(){return z.apply(null,arguments)}},He.cacheSignal=function(){return null},He.cloneElement=function(z,Y,ce){if(z==null)throw Error("The argument must be a React element, but you passed "+z+".");var de=w({},z.props),_e=z.key;if(Y!=null)for(F in Y.key!==void 0&&(_e=""+Y.key),Y)!q.call(Y,F)||F==="key"||F==="__self"||F==="__source"||F==="ref"&&Y.ref===void 0||(de[F]=Y[F]);var F=arguments.length-2;if(F===1)de.children=ce;else if(1<F){for(var G=Array(F),P=0;P<F;P++)G[P]=arguments[P+2];de.children=G}return Z(z.type,_e,de)},He.createContext=function(z){return z={$$typeof:c,_currentValue:z,_currentValue2:z,_threadCount:0,Provider:null,Consumer:null},z.Provider=z,z.Consumer={$$typeof:l,_context:z},z},He.createElement=function(z,Y,ce){var de,_e={},F=null;if(Y!=null)for(de in Y.key!==void 0&&(F=""+Y.key),Y)q.call(Y,de)&&de!=="key"&&de!=="__self"&&de!=="__source"&&(_e[de]=Y[de]);var G=arguments.length-2;if(G===1)_e.children=ce;else if(1<G){for(var P=Array(G),ae=0;ae<G;ae++)P[ae]=arguments[ae+2];_e.children=P}if(z&&z.defaultProps)for(de in G=z.defaultProps,G)_e[de]===void 0&&(_e[de]=G[de]);return Z(z,F,_e)},He.createRef=function(){return{current:null}},He.forwardRef=function(z){return{$$typeof:f,render:z}},He.isValidElement=Q,He.lazy=function(z){return{$$typeof:_,_payload:{_status:-1,_result:z},_init:B}},He.memo=function(z,Y){return{$$typeof:p,type:z,compare:Y===void 0?null:Y}},He.startTransition=function(z){var Y=R.T,ce={};R.T=ce;try{var de=z(),_e=R.S;_e!==null&&_e(ce,de),typeof de=="object"&&de!==null&&typeof de.then=="function"&&de.then(O,V)}catch(F){V(F)}finally{Y!==null&&ce.types!==null&&(Y.types=ce.types),R.T=Y}},He.unstable_useCacheRefresh=function(){return R.H.useCacheRefresh()},He.use=function(z){return R.H.use(z)},He.useActionState=function(z,Y,ce){return R.H.useActionState(z,Y,ce)},He.useCallback=function(z,Y){return R.H.useCallback(z,Y)},He.useContext=function(z){return R.H.useContext(z)},He.useDebugValue=function(){},He.useDeferredValue=function(z,Y){return R.H.useDeferredValue(z,Y)},He.useEffect=function(z,Y){return R.H.useEffect(z,Y)},He.useEffectEvent=function(z){return R.H.useEffectEvent(z)},He.useId=function(){return R.H.useId()},He.useImperativeHandle=function(z,Y,ce){return R.H.useImperativeHandle(z,Y,ce)},He.useInsertionEffect=function(z,Y){return R.H.useInsertionEffect(z,Y)},He.useLayoutEffect=function(z,Y){return R.H.useLayoutEffect(z,Y)},He.useMemo=function(z,Y){return R.H.useMemo(z,Y)},He.useOptimistic=function(z,Y){return R.H.useOptimistic(z,Y)},He.useReducer=function(z,Y,ce){return R.H.useReducer(z,Y,ce)},He.useRef=function(z){return R.H.useRef(z)},He.useState=function(z){return R.H.useState(z)},He.useSyncExternalStore=function(z,Y,ce){return R.H.useSyncExternalStore(z,Y,ce)},He.useTransition=function(){return R.H.useTransition()},He.version="19.2.6",He}var ov;function Wl(){return ov||(ov=1,lm.exports=MN()),lm.exports}var C=Wl();const kh=qx(C);var cm={exports:{}},kl={},um={exports:{}},dm={};var rv;function zN(){return rv||(rv=1,(function(e){function t(D,U){var B=D.length;D.push(U);e:for(;0<B;){var V=B-1>>>1,te=D[V];if(0<o(te,U))D[V]=U,D[B]=te,B=V;else break e}}function s(D){return D.length===0?null:D[0]}function a(D){if(D.length===0)return null;var U=D[0],B=D.pop();if(B!==U){D[0]=B;e:for(var V=0,te=D.length,z=te>>>1;V<z;){var Y=2*(V+1)-1,ce=D[Y],de=Y+1,_e=D[de];if(0>o(ce,B))de<te&&0>o(_e,ce)?(D[V]=_e,D[de]=B,V=de):(D[V]=ce,D[Y]=B,V=Y);else if(de<te&&0>o(_e,B))D[V]=_e,D[de]=B,V=de;else break e}}return U}function o(D,U){var B=D.sortIndex-U.sortIndex;return B!==0?B:D.id-U.id}if(e.unstable_now=void 0,typeof performance=="object"&&typeof performance.now=="function"){var l=performance;e.unstable_now=function(){return l.now()}}else{var c=Date,f=c.now();e.unstable_now=function(){return c.now()-f}}var m=[],p=[],_=1,y=null,b=3,v=!1,S=!1,w=!1,k=!1,j=typeof setTimeout=="function"?setTimeout:null,A=typeof clearTimeout=="function"?clearTimeout:null,N=typeof setImmediate<"u"?setImmediate:null;function T(D){for(var U=s(p);U!==null;){if(U.callback===null)a(p);else if(U.startTime<=D)a(p),U.sortIndex=U.expirationTime,t(m,U);else break;U=s(p)}}function E(D){if(w=!1,T(D),!S)if(s(m)!==null)S=!0,O||(O=!0,ee());else{var U=s(p);U!==null&&I(E,U.startTime-D)}}var O=!1,R=-1,q=5,Z=-1;function oe(){return k?!0:!(e.unstable_now()-Z<q)}function Q(){if(k=!1,O){var D=e.unstable_now();Z=D;var U=!0;try{e:{S=!1,w&&(w=!1,A(R),R=-1),v=!0;var B=b;try{t:{for(T(D),y=s(m);y!==null&&!(y.expirationTime>D&&oe());){var V=y.callback;if(typeof V=="function"){y.callback=null,b=y.priorityLevel;var te=V(y.expirationTime<=D);if(D=e.unstable_now(),typeof te=="function"){y.callback=te,T(D),U=!0;break t}y===s(m)&&a(m),T(D)}else a(m);y=s(m)}if(y!==null)U=!0;else{var z=s(p);z!==null&&I(E,z.startTime-D),U=!1}}break e}finally{y=null,b=B,v=!1}U=void 0}}finally{U?ee():O=!1}}}var ee;if(typeof N=="function")ee=function(){N(Q)};else if(typeof MessageChannel<"u"){var X=new MessageChannel,L=X.port2;X.port1.onmessage=Q,ee=function(){L.postMessage(null)}}else ee=function(){j(Q,0)};function I(D,U){R=j(function(){D(e.unstable_now())},U)}e.unstable_IdlePriority=5,e.unstable_ImmediatePriority=1,e.unstable_LowPriority=4,e.unstable_NormalPriority=3,e.unstable_Profiling=null,e.unstable_UserBlockingPriority=2,e.unstable_cancelCallback=function(D){D.callback=null},e.unstable_forceFrameRate=function(D){0>D||125<D?console.error("forceFrameRate takes a positive int between 0 and 125, forcing frame rates higher than 125 fps is not supported"):q=0<D?Math.floor(1e3/D):5},e.unstable_getCurrentPriorityLevel=function(){return b},e.unstable_next=function(D){switch(b){case 1:case 2:case 3:var U=3;break;default:U=b}var B=b;b=U;try{return D()}finally{b=B}},e.unstable_requestPaint=function(){k=!0},e.unstable_runWithPriority=function(D,U){switch(D){case 1:case 2:case 3:case 4:case 5:break;default:D=3}var B=b;b=D;try{return U()}finally{b=B}},e.unstable_scheduleCallback=function(D,U,B){var V=e.unstable_now();switch(typeof B=="object"&&B!==null?(B=B.delay,B=typeof B=="number"&&0<B?V+B:V):B=V,D){case 1:var te=-1;break;case 2:te=250;break;case 5:te=1073741823;break;case 4:te=1e4;break;default:te=5e3}return te=B+te,D={id:_++,callback:U,priorityLevel:D,startTime:B,expirationTime:te,sortIndex:-1},B>V?(D.sortIndex=B,t(p,D),s(m)===null&&D===s(p)&&(w?(A(R),R=-1):w=!0,I(E,B-V))):(D.sortIndex=te,t(m,D),S||v||(S=!0,O||(O=!0,ee()))),D},e.unstable_shouldYield=oe,e.unstable_wrapCallback=function(D){var U=b;return function(){var B=b;b=U;try{return D.apply(this,arguments)}finally{b=B}}}})(dm)),dm}var lv;function DN(){return lv||(lv=1,um.exports=zN()),um.exports}var fm={exports:{}},kn={};var cv;function $N(){if(cv)return kn;cv=1;var e=Wl();function t(m){var p="https://react.dev/errors/"+m;if(1<arguments.length){p+="?args[]="+encodeURIComponent(arguments[1]);for(var _=2;_<arguments.length;_++)p+="&args[]="+encodeURIComponent(arguments[_])}return"Minified React error #"+m+"; visit "+p+" for the full message or use the non-minified dev environment for full errors and additional helpful warnings."}function s(){}var a={d:{f:s,r:function(){throw Error(t(522))},D:s,C:s,L:s,m:s,X:s,S:s,M:s},p:0,findDOMNode:null},o=Symbol.for("react.portal");function l(m,p,_){var y=3<arguments.length&&arguments[3]!==void 0?arguments[3]:null;return{$$typeof:o,key:y==null?null:""+y,children:m,containerInfo:p,implementation:_}}var c=e.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE;function f(m,p){if(m==="font")return"";if(typeof p=="string")return p==="use-credentials"?p:""}return kn.__DOM_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE=a,kn.createPortal=function(m,p){var _=2<arguments.length&&arguments[2]!==void 0?arguments[2]:null;if(!p||p.nodeType!==1&&p.nodeType!==9&&p.nodeType!==11)throw Error(t(299));return l(m,p,null,_)},kn.flushSync=function(m){var p=c.T,_=a.p;try{if(c.T=null,a.p=2,m)return m()}finally{c.T=p,a.p=_,a.d.f()}},kn.preconnect=function(m,p){typeof m=="string"&&(p?(p=p.crossOrigin,p=typeof p=="string"?p==="use-credentials"?p:"":void 0):p=null,a.d.C(m,p))},kn.prefetchDNS=function(m){typeof m=="string"&&a.d.D(m)},kn.preinit=function(m,p){if(typeof m=="string"&&p&&typeof p.as=="string"){var _=p.as,y=f(_,p.crossOrigin),b=typeof p.integrity=="string"?p.integrity:void 0,v=typeof p.fetchPriority=="string"?p.fetchPriority:void 0;_==="style"?a.d.S(m,typeof p.precedence=="string"?p.precedence:void 0,{crossOrigin:y,integrity:b,fetchPriority:v}):_==="script"&&a.d.X(m,{crossOrigin:y,integrity:b,fetchPriority:v,nonce:typeof p.nonce=="string"?p.nonce:void 0})}},kn.preinitModule=function(m,p){if(typeof m=="string")if(typeof p=="object"&&p!==null){if(p.as==null||p.as==="script"){var _=f(p.as,p.crossOrigin);a.d.M(m,{crossOrigin:_,integrity:typeof p.integrity=="string"?p.integrity:void 0,nonce:typeof p.nonce=="string"?p.nonce:void 0})}}else p==null&&a.d.M(m)},kn.preload=function(m,p){if(typeof m=="string"&&typeof p=="object"&&p!==null&&typeof p.as=="string"){var _=p.as,y=f(_,p.crossOrigin);a.d.L(m,_,{crossOrigin:y,integrity:typeof p.integrity=="string"?p.integrity:void 0,nonce:typeof p.nonce=="string"?p.nonce:void 0,type:typeof p.type=="string"?p.type:void 0,fetchPriority:typeof p.fetchPriority=="string"?p.fetchPriority:void 0,referrerPolicy:typeof p.referrerPolicy=="string"?p.referrerPolicy:void 0,imageSrcSet:typeof p.imageSrcSet=="string"?p.imageSrcSet:void 0,imageSizes:typeof p.imageSizes=="string"?p.imageSizes:void 0,media:typeof p.media=="string"?p.media:void 0})}},kn.preloadModule=function(m,p){if(typeof m=="string")if(p){var _=f(p.as,p.crossOrigin);a.d.m(m,{as:typeof p.as=="string"&&p.as!=="script"?p.as:void 0,crossOrigin:_,integrity:typeof p.integrity=="string"?p.integrity:void 0})}else a.d.m(m)},kn.requestFormReset=function(m){a.d.r(m)},kn.unstable_batchedUpdates=function(m,p){return m(p)},kn.useFormState=function(m,p,_){return c.H.useFormState(m,p,_)},kn.useFormStatus=function(){return c.H.useHostTransitionStatus()},kn.version="19.2.6",kn}var uv;function Ux(){if(uv)return fm.exports;uv=1;function e(){if(!(typeof __REACT_DEVTOOLS_GLOBAL_HOOK__>"u"||typeof __REACT_DEVTOOLS_GLOBAL_HOOK__.checkDCE!="function"))try{__REACT_DEVTOOLS_GLOBAL_HOOK__.checkDCE(e)}catch(t){console.error(t)}}return e(),fm.exports=$N(),fm.exports}var dv;function RN(){if(dv)return kl;dv=1;var e=DN(),t=Wl(),s=Ux();function a(n){var i="https://react.dev/errors/"+n;if(1<arguments.length){i+="?args[]="+encodeURIComponent(arguments[1]);for(var r=2;r<arguments.length;r++)i+="&args[]="+encodeURIComponent(arguments[r])}return"Minified React error #"+n+"; visit "+i+" for the full message or use the non-minified dev environment for full errors and additional helpful warnings."}function o(n){return!(!n||n.nodeType!==1&&n.nodeType!==9&&n.nodeType!==11)}function l(n){var i=n,r=n;if(n.alternate)for(;i.return;)i=i.return;else{n=i;do i=n,(i.flags&4098)!==0&&(r=i.return),n=i.return;while(n)}return i.tag===3?r:null}function c(n){if(n.tag===13){var i=n.memoizedState;if(i===null&&(n=n.alternate,n!==null&&(i=n.memoizedState)),i!==null)return i.dehydrated}return null}function f(n){if(n.tag===31){var i=n.memoizedState;if(i===null&&(n=n.alternate,n!==null&&(i=n.memoizedState)),i!==null)return i.dehydrated}return null}function m(n){if(l(n)!==n)throw Error(a(188))}function p(n){var i=n.alternate;if(!i){if(i=l(n),i===null)throw Error(a(188));return i!==n?null:n}for(var r=n,u=i;;){var h=r.return;if(h===null)break;var g=h.alternate;if(g===null){if(u=h.return,u!==null){r=u;continue}break}if(h.child===g.child){for(g=h.child;g;){if(g===r)return m(h),n;if(g===u)return m(h),i;g=g.sibling}throw Error(a(188))}if(r.return!==u.return)r=h,u=g;else{for(var x=!1,M=h.child;M;){if(M===r){x=!0,r=h,u=g;break}if(M===u){x=!0,u=h,r=g;break}M=M.sibling}if(!x){for(M=g.child;M;){if(M===r){x=!0,r=g,u=h;break}if(M===u){x=!0,u=g,r=h;break}M=M.sibling}if(!x)throw Error(a(189))}}if(r.alternate!==u)throw Error(a(190))}if(r.tag!==3)throw Error(a(188));return r.stateNode.current===r?n:i}function _(n){var i=n.tag;if(i===5||i===26||i===27||i===6)return n;for(n=n.child;n!==null;){if(i=_(n),i!==null)return i;n=n.sibling}return null}var y=Object.assign,b=Symbol.for("react.element"),v=Symbol.for("react.transitional.element"),S=Symbol.for("react.portal"),w=Symbol.for("react.fragment"),k=Symbol.for("react.strict_mode"),j=Symbol.for("react.profiler"),A=Symbol.for("react.consumer"),N=Symbol.for("react.context"),T=Symbol.for("react.forward_ref"),E=Symbol.for("react.suspense"),O=Symbol.for("react.suspense_list"),R=Symbol.for("react.memo"),q=Symbol.for("react.lazy"),Z=Symbol.for("react.activity"),oe=Symbol.for("react.memo_cache_sentinel"),Q=Symbol.iterator;function ee(n){return n===null||typeof n!="object"?null:(n=Q&&n[Q]||n["@@iterator"],typeof n=="function"?n:null)}var X=Symbol.for("react.client.reference");function L(n){if(n==null)return null;if(typeof n=="function")return n.$$typeof===X?null:n.displayName||n.name||null;if(typeof n=="string")return n;switch(n){case w:return"Fragment";case j:return"Profiler";case k:return"StrictMode";case E:return"Suspense";case O:return"SuspenseList";case Z:return"Activity"}if(typeof n=="object")switch(n.$$typeof){case S:return"Portal";case N:return n.displayName||"Context";case A:return(n._context.displayName||"Context")+".Consumer";case T:var i=n.render;return n=n.displayName,n||(n=i.displayName||i.name||"",n=n!==""?"ForwardRef("+n+")":"ForwardRef"),n;case R:return i=n.displayName||null,i!==null?i:L(n.type)||"Memo";case q:i=n._payload,n=n._init;try{return L(n(i))}catch{}}return null}var I=Array.isArray,D=t.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE,U=s.__DOM_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE,B={pending:!1,data:null,method:null,action:null},V=[],te=-1;function z(n){return{current:n}}function Y(n){0>te||(n.current=V[te],V[te]=null,te--)}function ce(n,i){te++,V[te]=n.current,n.current=i}var de=z(null),_e=z(null),F=z(null),G=z(null);function P(n,i){switch(ce(F,i),ce(_e,n),ce(de,null),i.nodeType){case 9:case 11:n=(n=i.documentElement)&&(n=n.namespaceURI)?Nb(n):0;break;default:if(n=i.tagName,i=i.namespaceURI)i=Nb(i),n=Eb(i,n);else switch(n){case"svg":n=1;break;case"math":n=2;break;default:n=0}}Y(de),ce(de,n)}function ae(){Y(de),Y(_e),Y(F)}function Se(n){n.memoizedState!==null&&ce(G,n);var i=de.current,r=Eb(i,n.type);i!==r&&(ce(_e,n),ce(de,r))}function je(n){_e.current===n&&(Y(de),Y(_e)),G.current===n&&(Y(G),bl._currentValue=B)}var we,ve;function xe(n){if(we===void 0)try{throw Error()}catch(r){var i=r.stack.trim().match(/\n( *(at )?)/);we=i&&i[1]||"",ve=-1<r.stack.indexOf(`
    at`)?" (<anonymous>)":-1<r.stack.indexOf("@")?"@unknown:0:0":""}return`
`+we+n+ve}var ke=!1;function me(n,i){if(!n||ke)return"";ke=!0;var r=Error.prepareStackTrace;Error.prepareStackTrace=void 0;try{var u={DetermineComponentFrameRoot:function(){try{if(i){var ye=function(){throw Error()};if(Object.defineProperty(ye.prototype,"props",{set:function(){throw Error()}}),typeof Reflect=="object"&&Reflect.construct){try{Reflect.construct(ye,[])}catch(re){var ie=re}Reflect.construct(n,[],ye)}else{try{ye.call()}catch(re){ie=re}n.call(ye.prototype)}}else{try{throw Error()}catch(re){ie=re}(ye=n())&&typeof ye.catch=="function"&&ye.catch(function(){})}}catch(re){if(re&&ie&&typeof re.stack=="string")return[re.stack,ie.stack]}return[null,null]}};u.DetermineComponentFrameRoot.displayName="DetermineComponentFrameRoot";var h=Object.getOwnPropertyDescriptor(u.DetermineComponentFrameRoot,"name");h&&h.configurable&&Object.defineProperty(u.DetermineComponentFrameRoot,"name",{value:"DetermineComponentFrameRoot"});var g=u.DetermineComponentFrameRoot(),x=g[0],M=g[1];if(x&&M){var H=x.split(`
`),se=M.split(`
`);for(h=u=0;u<H.length&&!H[u].includes("DetermineComponentFrameRoot");)u++;for(;h<se.length&&!se[h].includes("DetermineComponentFrameRoot");)h++;if(u===H.length||h===se.length)for(u=H.length-1,h=se.length-1;1<=u&&0<=h&&H[u]!==se[h];)h--;for(;1<=u&&0<=h;u--,h--)if(H[u]!==se[h]){if(u!==1||h!==1)do if(u--,h--,0>h||H[u]!==se[h]){var fe=`
`+H[u].replace(" at new "," at ");return n.displayName&&fe.includes("<anonymous>")&&(fe=fe.replace("<anonymous>",n.displayName)),fe}while(1<=u&&0<=h);break}}}finally{ke=!1,Error.prepareStackTrace=r}return(r=n?n.displayName||n.name:"")?xe(r):""}function Ge(n,i){switch(n.tag){case 26:case 27:case 5:return xe(n.type);case 16:return xe("Lazy");case 13:return n.child!==i&&i!==null?xe("Suspense Fallback"):xe("Suspense");case 19:return xe("SuspenseList");case 0:case 15:return me(n.type,!1);case 11:return me(n.type.render,!1);case 1:return me(n.type,!0);case 31:return xe("Activity");default:return""}}function Ke(n){try{var i="",r=null;do i+=Ge(n,r),r=n,n=n.return;while(n);return i}catch(u){return`
Error generating stack: `+u.message+`
`+u.stack}}var rt=Object.prototype.hasOwnProperty,nt=e.unstable_scheduleCallback,qe=e.unstable_cancelCallback,Xt=e.unstable_shouldYield,Pt=e.unstable_requestPaint,lt=e.unstable_now,St=e.unstable_getCurrentPriorityLevel,mn=e.unstable_ImmediatePriority,Tt=e.unstable_UserBlockingPriority,Ft=e.unstable_NormalPriority,Jt=e.unstable_LowPriority,zn=e.unstable_IdlePriority,zi=e.log,Us=e.unstable_setDisableYieldValue,Dn=null,Qt=null;function Be(n){if(typeof zi=="function"&&Us(n),Qt&&typeof Qt.setStrictMode=="function")try{Qt.setStrictMode(Dn,n)}catch{}}var Mt=Math.clz32?Math.clz32:le,Bs=Math.log,va=Math.LN2;function le(n){return n>>>=0,n===0?32:31-(Bs(n)/va|0)|0}var Te=256,Re=262144,Ne=4194304;function Ce(n){var i=n&42;if(i!==0)return i;switch(n&-n){case 1:return 1;case 2:return 2;case 4:return 4;case 8:return 8;case 16:return 16;case 32:return 32;case 64:return 64;case 128:return 128;case 256:case 512:case 1024:case 2048:case 4096:case 8192:case 16384:case 32768:case 65536:case 131072:return n&261888;case 262144:case 524288:case 1048576:case 2097152:return n&3932160;case 4194304:case 8388608:case 16777216:case 33554432:return n&62914560;case 67108864:return 67108864;case 134217728:return 134217728;case 268435456:return 268435456;case 536870912:return 536870912;case 1073741824:return 0;default:return n}}function ct(n,i,r){var u=n.pendingLanes;if(u===0)return 0;var h=0,g=n.suspendedLanes,x=n.pingedLanes;n=n.warmLanes;var M=u&134217727;return M!==0?(u=M&~g,u!==0?h=Ce(u):(x&=M,x!==0?h=Ce(x):r||(r=M&~n,r!==0&&(h=Ce(r))))):(M=u&~g,M!==0?h=Ce(M):x!==0?h=Ce(x):r||(r=u&~n,r!==0&&(h=Ce(r)))),h===0?0:i!==0&&i!==h&&(i&g)===0&&(g=h&-h,r=i&-i,g>=r||g===32&&(r&4194048)!==0)?i:h}function wt(n,i){return(n.pendingLanes&~(n.suspendedLanes&~n.pingedLanes)&i)===0}function Vn(n,i){switch(n){case 1:case 2:case 4:case 8:case 64:return i+250;case 16:case 32:case 128:case 256:case 512:case 1024:case 2048:case 4096:case 8192:case 16384:case 32768:case 65536:case 131072:case 262144:case 524288:case 1048576:case 2097152:return i+5e3;case 4194304:case 8388608:case 16777216:case 33554432:return-1;case 67108864:case 134217728:case 268435456:case 536870912:case 1073741824:return-1;default:return-1}}function Ws(){var n=Ne;return Ne<<=1,(Ne&62914560)===0&&(Ne=4194304),n}function vs(n){for(var i=[],r=0;31>r;r++)i.push(n);return i}function Nn(n,i){n.pendingLanes|=i,i!==268435456&&(n.suspendedLanes=0,n.pingedLanes=0,n.warmLanes=0)}function Hs(n,i,r,u,h,g){var x=n.pendingLanes;n.pendingLanes=r,n.suspendedLanes=0,n.pingedLanes=0,n.warmLanes=0,n.expiredLanes&=r,n.entangledLanes&=r,n.errorRecoveryDisabledLanes&=r,n.shellSuspendCounter=0;var M=n.entanglements,H=n.expirationTimes,se=n.hiddenUpdates;for(r=x&~r;0<r;){var fe=31-Mt(r),ye=1<<fe;M[fe]=0,H[fe]=-1;var ie=se[fe];if(ie!==null)for(se[fe]=null,fe=0;fe<ie.length;fe++){var re=ie[fe];re!==null&&(re.lane&=-536870913)}r&=~ye}u!==0&&ei(n,u,0),g!==0&&h===0&&n.tag!==0&&(n.suspendedLanes|=g&~(x&~i))}function ei(n,i,r){n.pendingLanes|=i,n.suspendedLanes&=~i;var u=31-Mt(i);n.entangledLanes|=i,n.entanglements[u]=n.entanglements[u]|1073741824|r&261930}function xa(n,i){var r=n.entangledLanes|=i;for(n=n.entanglements;r;){var u=31-Mt(r),h=1<<u;h&i|n[u]&i&&(n[u]|=i),r&=~h}}function ti(n,i){var r=i&-i;return r=(r&42)!==0?1:Di(r),(r&(n.suspendedLanes|i))!==0?0:r}function Di(n){switch(n){case 2:n=1;break;case 8:n=4;break;case 32:n=16;break;case 256:case 512:case 1024:case 2048:case 4096:case 8192:case 16384:case 32768:case 65536:case 131072:case 262144:case 524288:case 1048576:case 2097152:case 4194304:case 8388608:case 16777216:case 33554432:n=128;break;case 268435456:n=134217728;break;default:n=0}return n}function ni(n){return n&=-n,2<n?8<n?(n&134217727)!==0?32:268435456:8:2}function wa(){var n=U.p;return n!==0?n:(n=window.event,n===void 0?32:Fb(n.type))}function Sa(n,i){var r=U.p;try{return U.p=n,i()}finally{U.p=r}}var En=Math.random().toString(36).slice(2),kt="__reactFiber$"+En,Yt="__reactProps$"+En,as="__reactContainer$"+En,xs="__reactEvents$"+En,$i="__reactListeners$"+En,Je="__reactHandles$"+En,hn="__reactResources$"+En,gn="__reactMarker$"+En;function yn(n){delete n[kt],delete n[Yt],delete n[xs],delete n[$i],delete n[Je]}function un(n){var i=n[kt];if(i)return i;for(var r=n.parentNode;r;){if(i=r[as]||r[kt]){if(r=i.alternate,i.child!==null||r!==null&&r.child!==null)for(n=Db(n);n!==null;){if(r=n[kt])return r;n=Db(n)}return i}n=r,r=n.parentNode}return null}function Tn(n){if(n=n[kt]||n[as]){var i=n.tag;if(i===5||i===6||i===13||i===31||i===26||i===27||i===3)return n}return null}function zt(n){var i=n.tag;if(i===5||i===26||i===27||i===6)return n.stateNode;throw Error(a(33))}function Cn(n){var i=n[hn];return i||(i=n[hn]={hoistableStyles:new Map,hoistableScripts:new Map}),i}function Dt(n){n[gn]=!0}var bo=new Set,ws={};function os(n,i){rs(n,i),rs(n+"Capture",i)}function rs(n,i){for(ws[n]=i,n=0;n<i.length;n++)bo.add(i[n])}var vo=RegExp("^[:A-Z_a-z\\u00C0-\\u00D6\\u00D8-\\u00F6\\u00F8-\\u02FF\\u0370-\\u037D\\u037F-\\u1FFF\\u200C-\\u200D\\u2070-\\u218F\\u2C00-\\u2FEF\\u3001-\\uD7FF\\uF900-\\uFDCF\\uFDF0-\\uFFFD][:A-Z_a-z\\u00C0-\\u00D6\\u00D8-\\u00F6\\u00F8-\\u02FF\\u0370-\\u037D\\u037F-\\u1FFF\\u200C-\\u200D\\u2070-\\u218F\\u2C00-\\u2FEF\\u3001-\\uD7FF\\uF900-\\uFDCF\\uFDF0-\\uFFFD\\-.0-9\\u00B7\\u0300-\\u036F\\u203F-\\u2040]*$"),ka={},ja={};function xo(n){return rt.call(ja,n)?!0:rt.call(ka,n)?!1:vo.test(n)?ja[n]=!0:(ka[n]=!0,!1)}function Ss(n,i,r){if(xo(i))if(r===null)n.removeAttribute(i);else{switch(typeof r){case"undefined":case"function":case"symbol":n.removeAttribute(i);return;case"boolean":var u=i.toLowerCase().slice(0,5);if(u!=="data-"&&u!=="aria-"){n.removeAttribute(i);return}}n.setAttribute(i,""+r)}}function si(n,i,r){if(r===null)n.removeAttribute(i);else{switch(typeof r){case"undefined":case"function":case"symbol":case"boolean":n.removeAttribute(i);return}n.setAttribute(i,""+r)}}function $n(n,i,r,u){if(u===null)n.removeAttribute(r);else{switch(typeof u){case"undefined":case"function":case"symbol":case"boolean":n.removeAttribute(r);return}n.setAttributeNS(i,r,""+u)}}function wn(n){switch(typeof n){case"bigint":case"boolean":case"number":case"string":case"undefined":return n;case"object":return n;default:return""}}function Ri(n){var i=n.type;return(n=n.nodeName)&&n.toLowerCase()==="input"&&(i==="checkbox"||i==="radio")}function Na(n,i,r){var u=Object.getOwnPropertyDescriptor(n.constructor.prototype,i);if(!n.hasOwnProperty(i)&&typeof u<"u"&&typeof u.get=="function"&&typeof u.set=="function"){var h=u.get,g=u.set;return Object.defineProperty(n,i,{configurable:!0,get:function(){return h.call(this)},set:function(x){r=""+x,g.call(this,x)}}),Object.defineProperty(n,i,{enumerable:u.enumerable}),{getValue:function(){return r},setValue:function(x){r=""+x},stopTracking:function(){n._valueTracker=null,delete n[i]}}}}function Ea(n){if(!n._valueTracker){var i=Ri(n)?"checked":"value";n._valueTracker=Na(n,i,""+n[i])}}function Ta(n){if(!n)return!1;var i=n._valueTracker;if(!i)return!0;var r=i.getValue(),u="";return n&&(u=Ri(n)?n.checked?"true":"false":n.value),n=u,n!==r?(i.setValue(n),!0):!1}function Li(n){if(n=n||(typeof document<"u"?document:void 0),typeof n>"u")return null;try{return n.activeElement||n.body}catch{return n.body}}var wo=/[\n"\\]/g;function Sn(n){return n.replace(wo,function(i){return"\\"+i.charCodeAt(0).toString(16)+" "})}function ii(n,i,r,u,h,g,x,M){n.name="",x!=null&&typeof x!="function"&&typeof x!="symbol"&&typeof x!="boolean"?n.type=x:n.removeAttribute("type"),i!=null?x==="number"?(i===0&&n.value===""||n.value!=i)&&(n.value=""+wn(i)):n.value!==""+wn(i)&&(n.value=""+wn(i)):x!=="submit"&&x!=="reset"||n.removeAttribute("value"),i!=null?Ca(n,x,wn(i)):r!=null?Ca(n,x,wn(r)):u!=null&&n.removeAttribute("value"),h==null&&g!=null&&(n.defaultChecked=!!g),h!=null&&(n.checked=h&&typeof h!="function"&&typeof h!="symbol"),M!=null&&typeof M!="function"&&typeof M!="symbol"&&typeof M!="boolean"?n.name=""+wn(M):n.removeAttribute("name")}function qi(n,i,r,u,h,g,x,M){if(g!=null&&typeof g!="function"&&typeof g!="symbol"&&typeof g!="boolean"&&(n.type=g),i!=null||r!=null){if(!(g!=="submit"&&g!=="reset"||i!=null)){Ea(n);return}r=r!=null?""+wn(r):"",i=i!=null?""+wn(i):r,M||i===n.value||(n.value=i),n.defaultValue=i}u=u??h,u=typeof u!="function"&&typeof u!="symbol"&&!!u,n.checked=M?n.checked:!!u,n.defaultChecked=!!u,x!=null&&typeof x!="function"&&typeof x!="symbol"&&typeof x!="boolean"&&(n.name=x),Ea(n)}function Ca(n,i,r){i==="number"&&Li(n.ownerDocument)===n||n.defaultValue===""+r||(n.defaultValue=""+r)}function Is(n,i,r,u){if(n=n.options,i){i={};for(var h=0;h<r.length;h++)i["$"+r[h]]=!0;for(r=0;r<n.length;r++)h=i.hasOwnProperty("$"+n[r].value),n[r].selected!==h&&(n[r].selected=h),h&&u&&(n[r].defaultSelected=!0)}else{for(r=""+wn(r),i=null,h=0;h<n.length;h++){if(n[h].value===r){n[h].selected=!0,u&&(n[h].defaultSelected=!0);return}i!==null||n[h].disabled||(i=n[h])}i!==null&&(i.selected=!0)}}function Ui(n,i,r){if(i!=null&&(i=""+wn(i),i!==n.value&&(n.value=i),r==null)){n.defaultValue!==i&&(n.defaultValue=i);return}n.defaultValue=r!=null?""+wn(r):""}function pc(n,i,r,u){if(i==null){if(u!=null){if(r!=null)throw Error(a(92));if(I(u)){if(1<u.length)throw Error(a(93));u=u[0]}r=u}r==null&&(r=""),i=r}r=wn(i),n.defaultValue=r,u=n.textContent,u===r&&u!==""&&u!==null&&(n.value=u),Ea(n)}function ai(n,i){if(i){var r=n.firstChild;if(r&&r===n.lastChild&&r.nodeType===3){r.nodeValue=i;return}}n.textContent=i}var mc=new Set("animationIterationCount aspectRatio borderImageOutset borderImageSlice borderImageWidth boxFlex boxFlexGroup boxOrdinalGroup columnCount columns flex flexGrow flexPositive flexShrink flexNegative flexOrder gridArea gridRow gridRowEnd gridRowSpan gridRowStart gridColumn gridColumnEnd gridColumnSpan gridColumnStart fontWeight lineClamp lineHeight opacity order orphans scale tabSize widows zIndex zoom fillOpacity floodOpacity stopOpacity strokeDasharray strokeDashoffset strokeMiterlimit strokeOpacity strokeWidth MozAnimationIterationCount MozBoxFlex MozBoxFlexGroup MozLineClamp msAnimationIterationCount msFlex msZoom msFlexGrow msFlexNegative msFlexOrder msFlexPositive msFlexShrink msGridColumn msGridColumnSpan msGridRow msGridRowSpan WebkitAnimationIterationCount WebkitBoxFlex WebKitBoxFlexGroup WebkitBoxOrdinalGroup WebkitColumnCount WebkitColumns WebkitFlex WebkitFlexGrow WebkitFlexPositive WebkitFlexShrink WebkitLineClamp".split(" "));function hc(n,i,r){var u=i.indexOf("--")===0;r==null||typeof r=="boolean"||r===""?u?n.setProperty(i,""):i==="float"?n.cssFloat="":n[i]="":u?n.setProperty(i,r):typeof r!="number"||r===0||mc.has(i)?i==="float"?n.cssFloat=r:n[i]=(""+r).trim():n[i]=r+"px"}function Lr(n,i,r){if(i!=null&&typeof i!="object")throw Error(a(62));if(n=n.style,r!=null){for(var u in r)!r.hasOwnProperty(u)||i!=null&&i.hasOwnProperty(u)||(u.indexOf("--")===0?n.setProperty(u,""):u==="float"?n.cssFloat="":n[u]="");for(var h in i)u=i[h],i.hasOwnProperty(h)&&r[h]!==u&&hc(n,h,u)}else for(var g in i)i.hasOwnProperty(g)&&hc(n,g,i[g])}function Aa(n){if(n.indexOf("-")===-1)return!1;switch(n){case"annotation-xml":case"color-profile":case"font-face":case"font-face-src":case"font-face-uri":case"font-face-format":case"font-face-name":case"missing-glyph":return!1;default:return!0}}var $=new Map([["acceptCharset","accept-charset"],["htmlFor","for"],["httpEquiv","http-equiv"],["crossOrigin","crossorigin"],["accentHeight","accent-height"],["alignmentBaseline","alignment-baseline"],["arabicForm","arabic-form"],["baselineShift","baseline-shift"],["capHeight","cap-height"],["clipPath","clip-path"],["clipRule","clip-rule"],["colorInterpolation","color-interpolation"],["colorInterpolationFilters","color-interpolation-filters"],["colorProfile","color-profile"],["colorRendering","color-rendering"],["dominantBaseline","dominant-baseline"],["enableBackground","enable-background"],["fillOpacity","fill-opacity"],["fillRule","fill-rule"],["floodColor","flood-color"],["floodOpacity","flood-opacity"],["fontFamily","font-family"],["fontSize","font-size"],["fontSizeAdjust","font-size-adjust"],["fontStretch","font-stretch"],["fontStyle","font-style"],["fontVariant","font-variant"],["fontWeight","font-weight"],["glyphName","glyph-name"],["glyphOrientationHorizontal","glyph-orientation-horizontal"],["glyphOrientationVertical","glyph-orientation-vertical"],["horizAdvX","horiz-adv-x"],["horizOriginX","horiz-origin-x"],["imageRendering","image-rendering"],["letterSpacing","letter-spacing"],["lightingColor","lighting-color"],["markerEnd","marker-end"],["markerMid","marker-mid"],["markerStart","marker-start"],["overlinePosition","overline-position"],["overlineThickness","overline-thickness"],["paintOrder","paint-order"],["panose-1","panose-1"],["pointerEvents","pointer-events"],["renderingIntent","rendering-intent"],["shapeRendering","shape-rendering"],["stopColor","stop-color"],["stopOpacity","stop-opacity"],["strikethroughPosition","strikethrough-position"],["strikethroughThickness","strikethrough-thickness"],["strokeDasharray","stroke-dasharray"],["strokeDashoffset","stroke-dashoffset"],["strokeLinecap","stroke-linecap"],["strokeLinejoin","stroke-linejoin"],["strokeMiterlimit","stroke-miterlimit"],["strokeOpacity","stroke-opacity"],["strokeWidth","stroke-width"],["textAnchor","text-anchor"],["textDecoration","text-decoration"],["textRendering","text-rendering"],["transformOrigin","transform-origin"],["underlinePosition","underline-position"],["underlineThickness","underline-thickness"],["unicodeBidi","unicode-bidi"],["unicodeRange","unicode-range"],["unitsPerEm","units-per-em"],["vAlphabetic","v-alphabetic"],["vHanging","v-hanging"],["vIdeographic","v-ideographic"],["vMathematical","v-mathematical"],["vectorEffect","vector-effect"],["vertAdvY","vert-adv-y"],["vertOriginX","vert-origin-x"],["vertOriginY","vert-origin-y"],["wordSpacing","word-spacing"],["writingMode","writing-mode"],["xmlnsXlink","xmlns:xlink"],["xHeight","x-height"]]),W=/^[\u0000-\u001F ]*j[\r\n\t]*a[\r\n\t]*v[\r\n\t]*a[\r\n\t]*s[\r\n\t]*c[\r\n\t]*r[\r\n\t]*i[\r\n\t]*p[\r\n\t]*t[\r\n\t]*:/i;function pe(n){return W.test(""+n)?"javascript:throw new Error('React has blocked a javascript: URL as a security precaution.')":n}function be(){}var Oe=null;function dn(n){return n=n.target||n.srcElement||window,n.correspondingUseElement&&(n=n.correspondingUseElement),n.nodeType===3?n.parentNode:n}var Ve=null,yt=null;function on(n){var i=Tn(n);if(i&&(n=i.stateNode)){var r=n[Yt]||null;e:switch(n=i.stateNode,i.type){case"input":if(ii(n,r.value,r.defaultValue,r.defaultValue,r.checked,r.defaultChecked,r.type,r.name),i=r.name,r.type==="radio"&&i!=null){for(r=n;r.parentNode;)r=r.parentNode;for(r=r.querySelectorAll('input[name="'+Sn(""+i)+'"][type="radio"]'),i=0;i<r.length;i++){var u=r[i];if(u!==n&&u.form===n.form){var h=u[Yt]||null;if(!h)throw Error(a(90));ii(u,h.value,h.defaultValue,h.defaultValue,h.checked,h.defaultChecked,h.type,h.name)}}for(i=0;i<r.length;i++)u=r[i],u.form===n.form&&Ta(u)}break e;case"textarea":Ui(n,r.value,r.defaultValue);break e;case"select":i=r.value,i!=null&&Is(n,!!r.multiple,i,!1)}}}var ks=!1;function oi(n,i,r){if(ks)return n(i,r);ks=!0;try{var u=n(i);return u}finally{if(ks=!1,(Ve!==null||yt!==null)&&(nu(),Ve&&(i=Ve,n=yt,yt=Ve=null,on(i),n)))for(i=0;i<n.length;i++)on(n[i])}}function Ht(n,i){var r=n.stateNode;if(r===null)return null;var u=r[Yt]||null;if(u===null)return null;r=u[i];e:switch(i){case"onClick":case"onClickCapture":case"onDoubleClick":case"onDoubleClickCapture":case"onMouseDown":case"onMouseDownCapture":case"onMouseMove":case"onMouseMoveCapture":case"onMouseUp":case"onMouseUpCapture":case"onMouseEnter":(u=!u.disabled)||(n=n.type,u=!(n==="button"||n==="input"||n==="select"||n==="textarea")),n=!u;break e;default:n=!1}if(n)return null;if(r&&typeof r!="function")throw Error(a(231,i,typeof r));return r}var Kn=!(typeof window>"u"||typeof window.document>"u"||typeof window.document.createElement>"u"),So=!1;if(Kn)try{var Oa={};Object.defineProperty(Oa,"passive",{get:function(){So=!0}}),window.addEventListener("test",Oa,Oa),window.removeEventListener("test",Oa,Oa)}catch{So=!1}var Bi=null,lf=null,gc=null;function Eg(){if(gc)return gc;var n,i=lf,r=i.length,u,h="value"in Bi?Bi.value:Bi.textContent,g=h.length;for(n=0;n<r&&i[n]===h[n];n++);var x=r-n;for(u=1;u<=x&&i[r-u]===h[g-u];u++);return gc=h.slice(n,1<u?1-u:void 0)}function yc(n){var i=n.keyCode;return"charCode"in n?(n=n.charCode,n===0&&i===13&&(n=13)):n=i,n===10&&(n=13),32<=n||n===13?n:0}function _c(){return!0}function Tg(){return!1}function Rn(n){function i(r,u,h,g,x){this._reactName=r,this._targetInst=h,this.type=u,this.nativeEvent=g,this.target=x,this.currentTarget=null;for(var M in n)n.hasOwnProperty(M)&&(r=n[M],this[M]=r?r(g):g[M]);return this.isDefaultPrevented=(g.defaultPrevented!=null?g.defaultPrevented:g.returnValue===!1)?_c:Tg,this.isPropagationStopped=Tg,this}return y(i.prototype,{preventDefault:function(){this.defaultPrevented=!0;var r=this.nativeEvent;r&&(r.preventDefault?r.preventDefault():typeof r.returnValue!="unknown"&&(r.returnValue=!1),this.isDefaultPrevented=_c)},stopPropagation:function(){var r=this.nativeEvent;r&&(r.stopPropagation?r.stopPropagation():typeof r.cancelBubble!="unknown"&&(r.cancelBubble=!0),this.isPropagationStopped=_c)},persist:function(){},isPersistent:_c}),i}var Ma={eventPhase:0,bubbles:0,cancelable:0,timeStamp:function(n){return n.timeStamp||Date.now()},defaultPrevented:0,isTrusted:0},bc=Rn(Ma),qr=y({},Ma,{view:0,detail:0}),Tk=Rn(qr),cf,uf,Ur,vc=y({},qr,{screenX:0,screenY:0,clientX:0,clientY:0,pageX:0,pageY:0,ctrlKey:0,shiftKey:0,altKey:0,metaKey:0,getModifierState:ff,button:0,buttons:0,relatedTarget:function(n){return n.relatedTarget===void 0?n.fromElement===n.srcElement?n.toElement:n.fromElement:n.relatedTarget},movementX:function(n){return"movementX"in n?n.movementX:(n!==Ur&&(Ur&&n.type==="mousemove"?(cf=n.screenX-Ur.screenX,uf=n.screenY-Ur.screenY):uf=cf=0,Ur=n),cf)},movementY:function(n){return"movementY"in n?n.movementY:uf}}),Cg=Rn(vc),Ck=y({},vc,{dataTransfer:0}),Ak=Rn(Ck),Ok=y({},qr,{relatedTarget:0}),df=Rn(Ok),Mk=y({},Ma,{animationName:0,elapsedTime:0,pseudoElement:0}),zk=Rn(Mk),Dk=y({},Ma,{clipboardData:function(n){return"clipboardData"in n?n.clipboardData:window.clipboardData}}),$k=Rn(Dk),Rk=y({},Ma,{data:0}),Ag=Rn(Rk),Lk={Esc:"Escape",Spacebar:" ",Left:"ArrowLeft",Up:"ArrowUp",Right:"ArrowRight",Down:"ArrowDown",Del:"Delete",Win:"OS",Menu:"ContextMenu",Apps:"ContextMenu",Scroll:"ScrollLock",MozPrintableKey:"Unidentified"},qk={8:"Backspace",9:"Tab",12:"Clear",13:"Enter",16:"Shift",17:"Control",18:"Alt",19:"Pause",20:"CapsLock",27:"Escape",32:" ",33:"PageUp",34:"PageDown",35:"End",36:"Home",37:"ArrowLeft",38:"ArrowUp",39:"ArrowRight",40:"ArrowDown",45:"Insert",46:"Delete",112:"F1",113:"F2",114:"F3",115:"F4",116:"F5",117:"F6",118:"F7",119:"F8",120:"F9",121:"F10",122:"F11",123:"F12",144:"NumLock",145:"ScrollLock",224:"Meta"},Uk={Alt:"altKey",Control:"ctrlKey",Meta:"metaKey",Shift:"shiftKey"};function Bk(n){var i=this.nativeEvent;return i.getModifierState?i.getModifierState(n):(n=Uk[n])?!!i[n]:!1}function ff(){return Bk}var Hk=y({},qr,{key:function(n){if(n.key){var i=Lk[n.key]||n.key;if(i!=="Unidentified")return i}return n.type==="keypress"?(n=yc(n),n===13?"Enter":String.fromCharCode(n)):n.type==="keydown"||n.type==="keyup"?qk[n.keyCode]||"Unidentified":""},code:0,location:0,ctrlKey:0,shiftKey:0,altKey:0,metaKey:0,repeat:0,locale:0,getModifierState:ff,charCode:function(n){return n.type==="keypress"?yc(n):0},keyCode:function(n){return n.type==="keydown"||n.type==="keyup"?n.keyCode:0},which:function(n){return n.type==="keypress"?yc(n):n.type==="keydown"||n.type==="keyup"?n.keyCode:0}}),Ik=Rn(Hk),Zk=y({},vc,{pointerId:0,width:0,height:0,pressure:0,tangentialPressure:0,tiltX:0,tiltY:0,twist:0,pointerType:0,isPrimary:0}),Og=Rn(Zk),Pk=y({},qr,{touches:0,targetTouches:0,changedTouches:0,altKey:0,metaKey:0,ctrlKey:0,shiftKey:0,getModifierState:ff}),Yk=Rn(Pk),Vk=y({},Ma,{propertyName:0,elapsedTime:0,pseudoElement:0}),Kk=Rn(Vk),Gk=y({},vc,{deltaX:function(n){return"deltaX"in n?n.deltaX:"wheelDeltaX"in n?-n.wheelDeltaX:0},deltaY:function(n){return"deltaY"in n?n.deltaY:"wheelDeltaY"in n?-n.wheelDeltaY:"wheelDelta"in n?-n.wheelDelta:0},deltaZ:0,deltaMode:0}),Xk=Rn(Gk),Fk=y({},Ma,{newState:0,oldState:0}),Jk=Rn(Fk),Qk=[9,13,27,32],pf=Kn&&"CompositionEvent"in window,Br=null;Kn&&"documentMode"in document&&(Br=document.documentMode);var Wk=Kn&&"TextEvent"in window&&!Br,Mg=Kn&&(!pf||Br&&8<Br&&11>=Br),zg=" ",Dg=!1;function $g(n,i){switch(n){case"keyup":return Qk.indexOf(i.keyCode)!==-1;case"keydown":return i.keyCode!==229;case"keypress":case"mousedown":case"focusout":return!0;default:return!1}}function Rg(n){return n=n.detail,typeof n=="object"&&"data"in n?n.data:null}var ko=!1;function ej(n,i){switch(n){case"compositionend":return Rg(i);case"keypress":return i.which!==32?null:(Dg=!0,zg);case"textInput":return n=i.data,n===zg&&Dg?null:n;default:return null}}function tj(n,i){if(ko)return n==="compositionend"||!pf&&$g(n,i)?(n=Eg(),gc=lf=Bi=null,ko=!1,n):null;switch(n){case"paste":return null;case"keypress":if(!(i.ctrlKey||i.altKey||i.metaKey)||i.ctrlKey&&i.altKey){if(i.char&&1<i.char.length)return i.char;if(i.which)return String.fromCharCode(i.which)}return null;case"compositionend":return Mg&&i.locale!=="ko"?null:i.data;default:return null}}var nj={color:!0,date:!0,datetime:!0,"datetime-local":!0,email:!0,month:!0,number:!0,password:!0,range:!0,search:!0,tel:!0,text:!0,time:!0,url:!0,week:!0};function Lg(n){var i=n&&n.nodeName&&n.nodeName.toLowerCase();return i==="input"?!!nj[n.type]:i==="textarea"}function qg(n,i,r,u){Ve?yt?yt.push(u):yt=[u]:Ve=u,i=cu(i,"onChange"),0<i.length&&(r=new bc("onChange","change",null,r,u),n.push({event:r,listeners:i}))}var Hr=null,Ir=null;function sj(n){vb(n,0)}function xc(n){var i=zt(n);if(Ta(i))return n}function Ug(n,i){if(n==="change")return i}var Bg=!1;if(Kn){var mf;if(Kn){var hf="oninput"in document;if(!hf){var Hg=document.createElement("div");Hg.setAttribute("oninput","return;"),hf=typeof Hg.oninput=="function"}mf=hf}else mf=!1;Bg=mf&&(!document.documentMode||9<document.documentMode)}function Ig(){Hr&&(Hr.detachEvent("onpropertychange",Zg),Ir=Hr=null)}function Zg(n){if(n.propertyName==="value"&&xc(Ir)){var i=[];qg(i,Ir,n,dn(n)),oi(sj,i)}}function ij(n,i,r){n==="focusin"?(Ig(),Hr=i,Ir=r,Hr.attachEvent("onpropertychange",Zg)):n==="focusout"&&Ig()}function aj(n){if(n==="selectionchange"||n==="keyup"||n==="keydown")return xc(Ir)}function oj(n,i){if(n==="click")return xc(i)}function rj(n,i){if(n==="input"||n==="change")return xc(i)}function lj(n,i){return n===i&&(n!==0||1/n===1/i)||n!==n&&i!==i}var Gn=typeof Object.is=="function"?Object.is:lj;function Zr(n,i){if(Gn(n,i))return!0;if(typeof n!="object"||n===null||typeof i!="object"||i===null)return!1;var r=Object.keys(n),u=Object.keys(i);if(r.length!==u.length)return!1;for(u=0;u<r.length;u++){var h=r[u];if(!rt.call(i,h)||!Gn(n[h],i[h]))return!1}return!0}function Pg(n){for(;n&&n.firstChild;)n=n.firstChild;return n}function Yg(n,i){var r=Pg(n);n=0;for(var u;r;){if(r.nodeType===3){if(u=n+r.textContent.length,n<=i&&u>=i)return{node:r,offset:i-n};n=u}e:{for(;r;){if(r.nextSibling){r=r.nextSibling;break e}r=r.parentNode}r=void 0}r=Pg(r)}}function Vg(n,i){return n&&i?n===i?!0:n&&n.nodeType===3?!1:i&&i.nodeType===3?Vg(n,i.parentNode):"contains"in n?n.contains(i):n.compareDocumentPosition?!!(n.compareDocumentPosition(i)&16):!1:!1}function Kg(n){n=n!=null&&n.ownerDocument!=null&&n.ownerDocument.defaultView!=null?n.ownerDocument.defaultView:window;for(var i=Li(n.document);i instanceof n.HTMLIFrameElement;){try{var r=typeof i.contentWindow.location.href=="string"}catch{r=!1}if(r)n=i.contentWindow;else break;i=Li(n.document)}return i}function gf(n){var i=n&&n.nodeName&&n.nodeName.toLowerCase();return i&&(i==="input"&&(n.type==="text"||n.type==="search"||n.type==="tel"||n.type==="url"||n.type==="password")||i==="textarea"||n.contentEditable==="true")}var cj=Kn&&"documentMode"in document&&11>=document.documentMode,jo=null,yf=null,Pr=null,_f=!1;function Gg(n,i,r){var u=r.window===r?r.document:r.nodeType===9?r:r.ownerDocument;_f||jo==null||jo!==Li(u)||(u=jo,"selectionStart"in u&&gf(u)?u={start:u.selectionStart,end:u.selectionEnd}:(u=(u.ownerDocument&&u.ownerDocument.defaultView||window).getSelection(),u={anchorNode:u.anchorNode,anchorOffset:u.anchorOffset,focusNode:u.focusNode,focusOffset:u.focusOffset}),Pr&&Zr(Pr,u)||(Pr=u,u=cu(yf,"onSelect"),0<u.length&&(i=new bc("onSelect","select",null,i,r),n.push({event:i,listeners:u}),i.target=jo)))}function za(n,i){var r={};return r[n.toLowerCase()]=i.toLowerCase(),r["Webkit"+n]="webkit"+i,r["Moz"+n]="moz"+i,r}var No={animationend:za("Animation","AnimationEnd"),animationiteration:za("Animation","AnimationIteration"),animationstart:za("Animation","AnimationStart"),transitionrun:za("Transition","TransitionRun"),transitionstart:za("Transition","TransitionStart"),transitioncancel:za("Transition","TransitionCancel"),transitionend:za("Transition","TransitionEnd")},bf={},Xg={};Kn&&(Xg=document.createElement("div").style,"AnimationEvent"in window||(delete No.animationend.animation,delete No.animationiteration.animation,delete No.animationstart.animation),"TransitionEvent"in window||delete No.transitionend.transition);function Da(n){if(bf[n])return bf[n];if(!No[n])return n;var i=No[n],r;for(r in i)if(i.hasOwnProperty(r)&&r in Xg)return bf[n]=i[r];return n}var Fg=Da("animationend"),Jg=Da("animationiteration"),Qg=Da("animationstart"),uj=Da("transitionrun"),dj=Da("transitionstart"),fj=Da("transitioncancel"),Wg=Da("transitionend"),ey=new Map,vf="abort auxClick beforeToggle cancel canPlay canPlayThrough click close contextMenu copy cut drag dragEnd dragEnter dragExit dragLeave dragOver dragStart drop durationChange emptied encrypted ended error gotPointerCapture input invalid keyDown keyPress keyUp load loadedData loadedMetadata loadStart lostPointerCapture mouseDown mouseMove mouseOut mouseOver mouseUp paste pause play playing pointerCancel pointerDown pointerMove pointerOut pointerOver pointerUp progress rateChange reset resize seeked seeking stalled submit suspend timeUpdate touchCancel touchEnd touchStart volumeChange scroll toggle touchMove waiting wheel".split(" ");vf.push("scrollEnd");function js(n,i){ey.set(n,i),os(i,[n])}var wc=typeof reportError=="function"?reportError:function(n){if(typeof window=="object"&&typeof window.ErrorEvent=="function"){var i=new window.ErrorEvent("error",{bubbles:!0,cancelable:!0,message:typeof n=="object"&&n!==null&&typeof n.message=="string"?String(n.message):String(n),error:n});if(!window.dispatchEvent(i))return}else if(typeof process=="object"&&typeof process.emit=="function"){process.emit("uncaughtException",n);return}console.error(n)},ls=[],Eo=0,xf=0;function Sc(){for(var n=Eo,i=xf=Eo=0;i<n;){var r=ls[i];ls[i++]=null;var u=ls[i];ls[i++]=null;var h=ls[i];ls[i++]=null;var g=ls[i];if(ls[i++]=null,u!==null&&h!==null){var x=u.pending;x===null?h.next=h:(h.next=x.next,x.next=h),u.pending=h}g!==0&&ty(r,h,g)}}function kc(n,i,r,u){ls[Eo++]=n,ls[Eo++]=i,ls[Eo++]=r,ls[Eo++]=u,xf|=u,n.lanes|=u,n=n.alternate,n!==null&&(n.lanes|=u)}function wf(n,i,r,u){return kc(n,i,r,u),jc(n)}function $a(n,i){return kc(n,null,null,i),jc(n)}function ty(n,i,r){n.lanes|=r;var u=n.alternate;u!==null&&(u.lanes|=r);for(var h=!1,g=n.return;g!==null;)g.childLanes|=r,u=g.alternate,u!==null&&(u.childLanes|=r),g.tag===22&&(n=g.stateNode,n===null||n._visibility&1||(h=!0)),n=g,g=g.return;return n.tag===3?(g=n.stateNode,h&&i!==null&&(h=31-Mt(r),n=g.hiddenUpdates,u=n[h],u===null?n[h]=[i]:u.push(i),i.lane=r|536870912),g):null}function jc(n){if(50<fl)throw fl=0,Op=null,Error(a(185));for(var i=n.return;i!==null;)n=i,i=n.return;return n.tag===3?n.stateNode:null}var To={};function pj(n,i,r,u){this.tag=n,this.key=r,this.sibling=this.child=this.return=this.stateNode=this.type=this.elementType=null,this.index=0,this.refCleanup=this.ref=null,this.pendingProps=i,this.dependencies=this.memoizedState=this.updateQueue=this.memoizedProps=null,this.mode=u,this.subtreeFlags=this.flags=0,this.deletions=null,this.childLanes=this.lanes=0,this.alternate=null}function Xn(n,i,r,u){return new pj(n,i,r,u)}function Sf(n){return n=n.prototype,!(!n||!n.isReactComponent)}function ri(n,i){var r=n.alternate;return r===null?(r=Xn(n.tag,i,n.key,n.mode),r.elementType=n.elementType,r.type=n.type,r.stateNode=n.stateNode,r.alternate=n,n.alternate=r):(r.pendingProps=i,r.type=n.type,r.flags=0,r.subtreeFlags=0,r.deletions=null),r.flags=n.flags&65011712,r.childLanes=n.childLanes,r.lanes=n.lanes,r.child=n.child,r.memoizedProps=n.memoizedProps,r.memoizedState=n.memoizedState,r.updateQueue=n.updateQueue,i=n.dependencies,r.dependencies=i===null?null:{lanes:i.lanes,firstContext:i.firstContext},r.sibling=n.sibling,r.index=n.index,r.ref=n.ref,r.refCleanup=n.refCleanup,r}function ny(n,i){n.flags&=65011714;var r=n.alternate;return r===null?(n.childLanes=0,n.lanes=i,n.child=null,n.subtreeFlags=0,n.memoizedProps=null,n.memoizedState=null,n.updateQueue=null,n.dependencies=null,n.stateNode=null):(n.childLanes=r.childLanes,n.lanes=r.lanes,n.child=r.child,n.subtreeFlags=0,n.deletions=null,n.memoizedProps=r.memoizedProps,n.memoizedState=r.memoizedState,n.updateQueue=r.updateQueue,n.type=r.type,i=r.dependencies,n.dependencies=i===null?null:{lanes:i.lanes,firstContext:i.firstContext}),n}function Nc(n,i,r,u,h,g){var x=0;if(u=n,typeof n=="function")Sf(n)&&(x=1);else if(typeof n=="string")x=_N(n,r,de.current)?26:n==="html"||n==="head"||n==="body"?27:5;else e:switch(n){case Z:return n=Xn(31,r,i,h),n.elementType=Z,n.lanes=g,n;case w:return Ra(r.children,h,g,i);case k:x=8,h|=24;break;case j:return n=Xn(12,r,i,h|2),n.elementType=j,n.lanes=g,n;case E:return n=Xn(13,r,i,h),n.elementType=E,n.lanes=g,n;case O:return n=Xn(19,r,i,h),n.elementType=O,n.lanes=g,n;default:if(typeof n=="object"&&n!==null)switch(n.$$typeof){case N:x=10;break e;case A:x=9;break e;case T:x=11;break e;case R:x=14;break e;case q:x=16,u=null;break e}x=29,r=Error(a(130,n===null?"null":typeof n,"")),u=null}return i=Xn(x,r,i,h),i.elementType=n,i.type=u,i.lanes=g,i}function Ra(n,i,r,u){return n=Xn(7,n,u,i),n.lanes=r,n}function kf(n,i,r){return n=Xn(6,n,null,i),n.lanes=r,n}function sy(n){var i=Xn(18,null,null,0);return i.stateNode=n,i}function jf(n,i,r){return i=Xn(4,n.children!==null?n.children:[],n.key,i),i.lanes=r,i.stateNode={containerInfo:n.containerInfo,pendingChildren:null,implementation:n.implementation},i}var iy=new WeakMap;function cs(n,i){if(typeof n=="object"&&n!==null){var r=iy.get(n);return r!==void 0?r:(i={value:n,source:i,stack:Ke(i)},iy.set(n,i),i)}return{value:n,source:i,stack:Ke(i)}}var Co=[],Ao=0,Ec=null,Yr=0,us=[],ds=0,Hi=null,Zs=1,Ps="";function li(n,i){Co[Ao++]=Yr,Co[Ao++]=Ec,Ec=n,Yr=i}function ay(n,i,r){us[ds++]=Zs,us[ds++]=Ps,us[ds++]=Hi,Hi=n;var u=Zs;n=Ps;var h=32-Mt(u)-1;u&=~(1<<h),r+=1;var g=32-Mt(i)+h;if(30<g){var x=h-h%5;g=(u&(1<<x)-1).toString(32),u>>=x,h-=x,Zs=1<<32-Mt(i)+h|r<<h|u,Ps=g+n}else Zs=1<<g|r<<h|u,Ps=n}function Nf(n){n.return!==null&&(li(n,1),ay(n,1,0))}function Ef(n){for(;n===Ec;)Ec=Co[--Ao],Co[Ao]=null,Yr=Co[--Ao],Co[Ao]=null;for(;n===Hi;)Hi=us[--ds],us[ds]=null,Ps=us[--ds],us[ds]=null,Zs=us[--ds],us[ds]=null}function oy(n,i){us[ds++]=Zs,us[ds++]=Ps,us[ds++]=Hi,Zs=i.id,Ps=i.overflow,Hi=n}var _n=null,jt=null,et=!1,Ii=null,fs=!1,Tf=Error(a(519));function Zi(n){var i=Error(a(418,1<arguments.length&&arguments[1]!==void 0&&arguments[1]?"text":"HTML",""));throw Vr(cs(i,n)),Tf}function ry(n){var i=n.stateNode,r=n.type,u=n.memoizedProps;switch(i[kt]=n,i[Yt]=u,r){case"dialog":Fe("cancel",i),Fe("close",i);break;case"iframe":case"object":case"embed":Fe("load",i);break;case"video":case"audio":for(r=0;r<ml.length;r++)Fe(ml[r],i);break;case"source":Fe("error",i);break;case"img":case"image":case"link":Fe("error",i),Fe("load",i);break;case"details":Fe("toggle",i);break;case"input":Fe("invalid",i),qi(i,u.value,u.defaultValue,u.checked,u.defaultChecked,u.type,u.name,!0);break;case"select":Fe("invalid",i);break;case"textarea":Fe("invalid",i),pc(i,u.value,u.defaultValue,u.children)}r=u.children,typeof r!="string"&&typeof r!="number"&&typeof r!="bigint"||i.textContent===""+r||u.suppressHydrationWarning===!0||kb(i.textContent,r)?(u.popover!=null&&(Fe("beforetoggle",i),Fe("toggle",i)),u.onScroll!=null&&Fe("scroll",i),u.onScrollEnd!=null&&Fe("scrollend",i),u.onClick!=null&&(i.onclick=be),i=!0):i=!1,i||Zi(n,!0)}function ly(n){for(_n=n.return;_n;)switch(_n.tag){case 5:case 31:case 13:fs=!1;return;case 27:case 3:fs=!0;return;default:_n=_n.return}}function Oo(n){if(n!==_n)return!1;if(!et)return ly(n),et=!0,!1;var i=n.tag,r;if((r=i!==3&&i!==27)&&((r=i===5)&&(r=n.type,r=!(r!=="form"&&r!=="button")||Vp(n.type,n.memoizedProps)),r=!r),r&&jt&&Zi(n),ly(n),i===13){if(n=n.memoizedState,n=n!==null?n.dehydrated:null,!n)throw Error(a(317));jt=zb(n)}else if(i===31){if(n=n.memoizedState,n=n!==null?n.dehydrated:null,!n)throw Error(a(317));jt=zb(n)}else i===27?(i=jt,sa(n.type)?(n=Jp,Jp=null,jt=n):jt=i):jt=_n?ms(n.stateNode.nextSibling):null;return!0}function La(){jt=_n=null,et=!1}function Cf(){var n=Ii;return n!==null&&(Bn===null?Bn=n:Bn.push.apply(Bn,n),Ii=null),n}function Vr(n){Ii===null?Ii=[n]:Ii.push(n)}var Af=z(null),qa=null,ci=null;function Pi(n,i,r){ce(Af,i._currentValue),i._currentValue=r}function ui(n){n._currentValue=Af.current,Y(Af)}function Of(n,i,r){for(;n!==null;){var u=n.alternate;if((n.childLanes&i)!==i?(n.childLanes|=i,u!==null&&(u.childLanes|=i)):u!==null&&(u.childLanes&i)!==i&&(u.childLanes|=i),n===r)break;n=n.return}}function Mf(n,i,r,u){var h=n.child;for(h!==null&&(h.return=n);h!==null;){var g=h.dependencies;if(g!==null){var x=h.child;g=g.firstContext;e:for(;g!==null;){var M=g;g=h;for(var H=0;H<i.length;H++)if(M.context===i[H]){g.lanes|=r,M=g.alternate,M!==null&&(M.lanes|=r),Of(g.return,r,n),u||(x=null);break e}g=M.next}}else if(h.tag===18){if(x=h.return,x===null)throw Error(a(341));x.lanes|=r,g=x.alternate,g!==null&&(g.lanes|=r),Of(x,r,n),x=null}else x=h.child;if(x!==null)x.return=h;else for(x=h;x!==null;){if(x===n){x=null;break}if(h=x.sibling,h!==null){h.return=x.return,x=h;break}x=x.return}h=x}}function Mo(n,i,r,u){n=null;for(var h=i,g=!1;h!==null;){if(!g){if((h.flags&524288)!==0)g=!0;else if((h.flags&262144)!==0)break}if(h.tag===10){var x=h.alternate;if(x===null)throw Error(a(387));if(x=x.memoizedProps,x!==null){var M=h.type;Gn(h.pendingProps.value,x.value)||(n!==null?n.push(M):n=[M])}}else if(h===G.current){if(x=h.alternate,x===null)throw Error(a(387));x.memoizedState.memoizedState!==h.memoizedState.memoizedState&&(n!==null?n.push(bl):n=[bl])}h=h.return}n!==null&&Mf(i,n,r,u),i.flags|=262144}function Tc(n){for(n=n.firstContext;n!==null;){if(!Gn(n.context._currentValue,n.memoizedValue))return!0;n=n.next}return!1}function Ua(n){qa=n,ci=null,n=n.dependencies,n!==null&&(n.firstContext=null)}function bn(n){return cy(qa,n)}function Cc(n,i){return qa===null&&Ua(n),cy(n,i)}function cy(n,i){var r=i._currentValue;if(i={context:i,memoizedValue:r,next:null},ci===null){if(n===null)throw Error(a(308));ci=i,n.dependencies={lanes:0,firstContext:i},n.flags|=524288}else ci=ci.next=i;return r}var mj=typeof AbortController<"u"?AbortController:function(){var n=[],i=this.signal={aborted:!1,addEventListener:function(r,u){n.push(u)}};this.abort=function(){i.aborted=!0,n.forEach(function(r){return r()})}},hj=e.unstable_scheduleCallback,gj=e.unstable_NormalPriority,Wt={$$typeof:N,Consumer:null,Provider:null,_currentValue:null,_currentValue2:null,_threadCount:0};function zf(){return{controller:new mj,data:new Map,refCount:0}}function Kr(n){n.refCount--,n.refCount===0&&hj(gj,function(){n.controller.abort()})}var Gr=null,Df=0,zo=0,Do=null;function yj(n,i){if(Gr===null){var r=Gr=[];Df=0,zo=Lp(),Do={status:"pending",value:void 0,then:function(u){r.push(u)}}}return Df++,i.then(uy,uy),i}function uy(){if(--Df===0&&Gr!==null){Do!==null&&(Do.status="fulfilled");var n=Gr;Gr=null,zo=0,Do=null;for(var i=0;i<n.length;i++)(0,n[i])()}}function _j(n,i){var r=[],u={status:"pending",value:null,reason:null,then:function(h){r.push(h)}};return n.then(function(){u.status="fulfilled",u.value=i;for(var h=0;h<r.length;h++)(0,r[h])(i)},function(h){for(u.status="rejected",u.reason=h,h=0;h<r.length;h++)(0,r[h])(void 0)}),u}var dy=D.S;D.S=function(n,i){G_=lt(),typeof i=="object"&&i!==null&&typeof i.then=="function"&&yj(n,i),dy!==null&&dy(n,i)};var Ba=z(null);function $f(){var n=Ba.current;return n!==null?n:_t.pooledCache}function Ac(n,i){i===null?ce(Ba,Ba.current):ce(Ba,i.pool)}function fy(){var n=$f();return n===null?null:{parent:Wt._currentValue,pool:n}}var $o=Error(a(460)),Rf=Error(a(474)),Oc=Error(a(542)),Mc={then:function(){}};function py(n){return n=n.status,n==="fulfilled"||n==="rejected"}function my(n,i,r){switch(r=n[r],r===void 0?n.push(i):r!==i&&(i.then(be,be),i=r),i.status){case"fulfilled":return i.value;case"rejected":throw n=i.reason,gy(n),n;default:if(typeof i.status=="string")i.then(be,be);else{if(n=_t,n!==null&&100<n.shellSuspendCounter)throw Error(a(482));n=i,n.status="pending",n.then(function(u){if(i.status==="pending"){var h=i;h.status="fulfilled",h.value=u}},function(u){if(i.status==="pending"){var h=i;h.status="rejected",h.reason=u}})}switch(i.status){case"fulfilled":return i.value;case"rejected":throw n=i.reason,gy(n),n}throw Ia=i,$o}}function Ha(n){try{var i=n._init;return i(n._payload)}catch(r){throw r!==null&&typeof r=="object"&&typeof r.then=="function"?(Ia=r,$o):r}}var Ia=null;function hy(){if(Ia===null)throw Error(a(459));var n=Ia;return Ia=null,n}function gy(n){if(n===$o||n===Oc)throw Error(a(483))}var Ro=null,Xr=0;function zc(n){var i=Xr;return Xr+=1,Ro===null&&(Ro=[]),my(Ro,n,i)}function Fr(n,i){i=i.props.ref,n.ref=i!==void 0?i:null}function Dc(n,i){throw i.$$typeof===b?Error(a(525)):(n=Object.prototype.toString.call(i),Error(a(31,n==="[object Object]"?"object with keys {"+Object.keys(i).join(", ")+"}":n)))}function yy(n){function i(J,K){if(n){var ne=J.deletions;ne===null?(J.deletions=[K],J.flags|=16):ne.push(K)}}function r(J,K){if(!n)return null;for(;K!==null;)i(J,K),K=K.sibling;return null}function u(J){for(var K=new Map;J!==null;)J.key!==null?K.set(J.key,J):K.set(J.index,J),J=J.sibling;return K}function h(J,K){return J=ri(J,K),J.index=0,J.sibling=null,J}function g(J,K,ne){return J.index=ne,n?(ne=J.alternate,ne!==null?(ne=ne.index,ne<K?(J.flags|=67108866,K):ne):(J.flags|=67108866,K)):(J.flags|=1048576,K)}function x(J){return n&&J.alternate===null&&(J.flags|=67108866),J}function M(J,K,ne,he){return K===null||K.tag!==6?(K=kf(ne,J.mode,he),K.return=J,K):(K=h(K,ne),K.return=J,K)}function H(J,K,ne,he){var $e=ne.type;return $e===w?fe(J,K,ne.props.children,he,ne.key):K!==null&&(K.elementType===$e||typeof $e=="object"&&$e!==null&&$e.$$typeof===q&&Ha($e)===K.type)?(K=h(K,ne.props),Fr(K,ne),K.return=J,K):(K=Nc(ne.type,ne.key,ne.props,null,J.mode,he),Fr(K,ne),K.return=J,K)}function se(J,K,ne,he){return K===null||K.tag!==4||K.stateNode.containerInfo!==ne.containerInfo||K.stateNode.implementation!==ne.implementation?(K=jf(ne,J.mode,he),K.return=J,K):(K=h(K,ne.children||[]),K.return=J,K)}function fe(J,K,ne,he,$e){return K===null||K.tag!==7?(K=Ra(ne,J.mode,he,$e),K.return=J,K):(K=h(K,ne),K.return=J,K)}function ye(J,K,ne){if(typeof K=="string"&&K!==""||typeof K=="number"||typeof K=="bigint")return K=kf(""+K,J.mode,ne),K.return=J,K;if(typeof K=="object"&&K!==null){switch(K.$$typeof){case v:return ne=Nc(K.type,K.key,K.props,null,J.mode,ne),Fr(ne,K),ne.return=J,ne;case S:return K=jf(K,J.mode,ne),K.return=J,K;case q:return K=Ha(K),ye(J,K,ne)}if(I(K)||ee(K))return K=Ra(K,J.mode,ne,null),K.return=J,K;if(typeof K.then=="function")return ye(J,zc(K),ne);if(K.$$typeof===N)return ye(J,Cc(J,K),ne);Dc(J,K)}return null}function ie(J,K,ne,he){var $e=K!==null?K.key:null;if(typeof ne=="string"&&ne!==""||typeof ne=="number"||typeof ne=="bigint")return $e!==null?null:M(J,K,""+ne,he);if(typeof ne=="object"&&ne!==null){switch(ne.$$typeof){case v:return ne.key===$e?H(J,K,ne,he):null;case S:return ne.key===$e?se(J,K,ne,he):null;case q:return ne=Ha(ne),ie(J,K,ne,he)}if(I(ne)||ee(ne))return $e!==null?null:fe(J,K,ne,he,null);if(typeof ne.then=="function")return ie(J,K,zc(ne),he);if(ne.$$typeof===N)return ie(J,K,Cc(J,ne),he);Dc(J,ne)}return null}function re(J,K,ne,he,$e){if(typeof he=="string"&&he!==""||typeof he=="number"||typeof he=="bigint")return J=J.get(ne)||null,M(K,J,""+he,$e);if(typeof he=="object"&&he!==null){switch(he.$$typeof){case v:return J=J.get(he.key===null?ne:he.key)||null,H(K,J,he,$e);case S:return J=J.get(he.key===null?ne:he.key)||null,se(K,J,he,$e);case q:return he=Ha(he),re(J,K,ne,he,$e)}if(I(he)||ee(he))return J=J.get(ne)||null,fe(K,J,he,$e,null);if(typeof he.then=="function")return re(J,K,ne,zc(he),$e);if(he.$$typeof===N)return re(J,K,ne,Cc(K,he),$e);Dc(K,he)}return null}function Ee(J,K,ne,he){for(var $e=null,at=null,Ae=K,Ye=K=0,We=null;Ae!==null&&Ye<ne.length;Ye++){Ae.index>Ye?(We=Ae,Ae=null):We=Ae.sibling;var ot=ie(J,Ae,ne[Ye],he);if(ot===null){Ae===null&&(Ae=We);break}n&&Ae&&ot.alternate===null&&i(J,Ae),K=g(ot,K,Ye),at===null?$e=ot:at.sibling=ot,at=ot,Ae=We}if(Ye===ne.length)return r(J,Ae),et&&li(J,Ye),$e;if(Ae===null){for(;Ye<ne.length;Ye++)Ae=ye(J,ne[Ye],he),Ae!==null&&(K=g(Ae,K,Ye),at===null?$e=Ae:at.sibling=Ae,at=Ae);return et&&li(J,Ye),$e}for(Ae=u(Ae);Ye<ne.length;Ye++)We=re(Ae,J,Ye,ne[Ye],he),We!==null&&(n&&We.alternate!==null&&Ae.delete(We.key===null?Ye:We.key),K=g(We,K,Ye),at===null?$e=We:at.sibling=We,at=We);return n&&Ae.forEach(function(la){return i(J,la)}),et&&li(J,Ye),$e}function Le(J,K,ne,he){if(ne==null)throw Error(a(151));for(var $e=null,at=null,Ae=K,Ye=K=0,We=null,ot=ne.next();Ae!==null&&!ot.done;Ye++,ot=ne.next()){Ae.index>Ye?(We=Ae,Ae=null):We=Ae.sibling;var la=ie(J,Ae,ot.value,he);if(la===null){Ae===null&&(Ae=We);break}n&&Ae&&la.alternate===null&&i(J,Ae),K=g(la,K,Ye),at===null?$e=la:at.sibling=la,at=la,Ae=We}if(ot.done)return r(J,Ae),et&&li(J,Ye),$e;if(Ae===null){for(;!ot.done;Ye++,ot=ne.next())ot=ye(J,ot.value,he),ot!==null&&(K=g(ot,K,Ye),at===null?$e=ot:at.sibling=ot,at=ot);return et&&li(J,Ye),$e}for(Ae=u(Ae);!ot.done;Ye++,ot=ne.next())ot=re(Ae,J,Ye,ot.value,he),ot!==null&&(n&&ot.alternate!==null&&Ae.delete(ot.key===null?Ye:ot.key),K=g(ot,K,Ye),at===null?$e=ot:at.sibling=ot,at=ot);return n&&Ae.forEach(function(CN){return i(J,CN)}),et&&li(J,Ye),$e}function gt(J,K,ne,he){if(typeof ne=="object"&&ne!==null&&ne.type===w&&ne.key===null&&(ne=ne.props.children),typeof ne=="object"&&ne!==null){switch(ne.$$typeof){case v:e:{for(var $e=ne.key;K!==null;){if(K.key===$e){if($e=ne.type,$e===w){if(K.tag===7){r(J,K.sibling),he=h(K,ne.props.children),he.return=J,J=he;break e}}else if(K.elementType===$e||typeof $e=="object"&&$e!==null&&$e.$$typeof===q&&Ha($e)===K.type){r(J,K.sibling),he=h(K,ne.props),Fr(he,ne),he.return=J,J=he;break e}r(J,K);break}else i(J,K);K=K.sibling}ne.type===w?(he=Ra(ne.props.children,J.mode,he,ne.key),he.return=J,J=he):(he=Nc(ne.type,ne.key,ne.props,null,J.mode,he),Fr(he,ne),he.return=J,J=he)}return x(J);case S:e:{for($e=ne.key;K!==null;){if(K.key===$e)if(K.tag===4&&K.stateNode.containerInfo===ne.containerInfo&&K.stateNode.implementation===ne.implementation){r(J,K.sibling),he=h(K,ne.children||[]),he.return=J,J=he;break e}else{r(J,K);break}else i(J,K);K=K.sibling}he=jf(ne,J.mode,he),he.return=J,J=he}return x(J);case q:return ne=Ha(ne),gt(J,K,ne,he)}if(I(ne))return Ee(J,K,ne,he);if(ee(ne)){if($e=ee(ne),typeof $e!="function")throw Error(a(150));return ne=$e.call(ne),Le(J,K,ne,he)}if(typeof ne.then=="function")return gt(J,K,zc(ne),he);if(ne.$$typeof===N)return gt(J,K,Cc(J,ne),he);Dc(J,ne)}return typeof ne=="string"&&ne!==""||typeof ne=="number"||typeof ne=="bigint"?(ne=""+ne,K!==null&&K.tag===6?(r(J,K.sibling),he=h(K,ne),he.return=J,J=he):(r(J,K),he=kf(ne,J.mode,he),he.return=J,J=he),x(J)):r(J,K)}return function(J,K,ne,he){try{Xr=0;var $e=gt(J,K,ne,he);return Ro=null,$e}catch(Ae){if(Ae===$o||Ae===Oc)throw Ae;var at=Xn(29,Ae,null,J.mode);return at.lanes=he,at.return=J,at}}}var Za=yy(!0),_y=yy(!1),Yi=!1;function Lf(n){n.updateQueue={baseState:n.memoizedState,firstBaseUpdate:null,lastBaseUpdate:null,shared:{pending:null,lanes:0,hiddenCallbacks:null},callbacks:null}}function qf(n,i){n=n.updateQueue,i.updateQueue===n&&(i.updateQueue={baseState:n.baseState,firstBaseUpdate:n.firstBaseUpdate,lastBaseUpdate:n.lastBaseUpdate,shared:n.shared,callbacks:null})}function Vi(n){return{lane:n,tag:0,payload:null,callback:null,next:null}}function Ki(n,i,r){var u=n.updateQueue;if(u===null)return null;if(u=u.shared,(ut&2)!==0){var h=u.pending;return h===null?i.next=i:(i.next=h.next,h.next=i),u.pending=i,i=jc(n),ty(n,null,r),i}return kc(n,u,i,r),jc(n)}function Jr(n,i,r){if(i=i.updateQueue,i!==null&&(i=i.shared,(r&4194048)!==0)){var u=i.lanes;u&=n.pendingLanes,r|=u,i.lanes=r,xa(n,r)}}function Uf(n,i){var r=n.updateQueue,u=n.alternate;if(u!==null&&(u=u.updateQueue,r===u)){var h=null,g=null;if(r=r.firstBaseUpdate,r!==null){do{var x={lane:r.lane,tag:r.tag,payload:r.payload,callback:null,next:null};g===null?h=g=x:g=g.next=x,r=r.next}while(r!==null);g===null?h=g=i:g=g.next=i}else h=g=i;r={baseState:u.baseState,firstBaseUpdate:h,lastBaseUpdate:g,shared:u.shared,callbacks:u.callbacks},n.updateQueue=r;return}n=r.lastBaseUpdate,n===null?r.firstBaseUpdate=i:n.next=i,r.lastBaseUpdate=i}var Bf=!1;function Qr(){if(Bf){var n=Do;if(n!==null)throw n}}function Wr(n,i,r,u){Bf=!1;var h=n.updateQueue;Yi=!1;var g=h.firstBaseUpdate,x=h.lastBaseUpdate,M=h.shared.pending;if(M!==null){h.shared.pending=null;var H=M,se=H.next;H.next=null,x===null?g=se:x.next=se,x=H;var fe=n.alternate;fe!==null&&(fe=fe.updateQueue,M=fe.lastBaseUpdate,M!==x&&(M===null?fe.firstBaseUpdate=se:M.next=se,fe.lastBaseUpdate=H))}if(g!==null){var ye=h.baseState;x=0,fe=se=H=null,M=g;do{var ie=M.lane&-536870913,re=ie!==M.lane;if(re?(Qe&ie)===ie:(u&ie)===ie){ie!==0&&ie===zo&&(Bf=!0),fe!==null&&(fe=fe.next={lane:0,tag:M.tag,payload:M.payload,callback:null,next:null});e:{var Ee=n,Le=M;ie=i;var gt=r;switch(Le.tag){case 1:if(Ee=Le.payload,typeof Ee=="function"){ye=Ee.call(gt,ye,ie);break e}ye=Ee;break e;case 3:Ee.flags=Ee.flags&-65537|128;case 0:if(Ee=Le.payload,ie=typeof Ee=="function"?Ee.call(gt,ye,ie):Ee,ie==null)break e;ye=y({},ye,ie);break e;case 2:Yi=!0}}ie=M.callback,ie!==null&&(n.flags|=64,re&&(n.flags|=8192),re=h.callbacks,re===null?h.callbacks=[ie]:re.push(ie))}else re={lane:ie,tag:M.tag,payload:M.payload,callback:M.callback,next:null},fe===null?(se=fe=re,H=ye):fe=fe.next=re,x|=ie;if(M=M.next,M===null){if(M=h.shared.pending,M===null)break;re=M,M=re.next,re.next=null,h.lastBaseUpdate=re,h.shared.pending=null}}while(!0);fe===null&&(H=ye),h.baseState=H,h.firstBaseUpdate=se,h.lastBaseUpdate=fe,g===null&&(h.shared.lanes=0),Qi|=x,n.lanes=x,n.memoizedState=ye}}function by(n,i){if(typeof n!="function")throw Error(a(191,n));n.call(i)}function vy(n,i){var r=n.callbacks;if(r!==null)for(n.callbacks=null,n=0;n<r.length;n++)by(r[n],i)}var Lo=z(null),$c=z(0);function xy(n,i){n=bi,ce($c,n),ce(Lo,i),bi=n|i.baseLanes}function Hf(){ce($c,bi),ce(Lo,Lo.current)}function If(){bi=$c.current,Y(Lo),Y($c)}var Fn=z(null),ps=null;function Gi(n){var i=n.alternate;ce(Vt,Vt.current&1),ce(Fn,n),ps===null&&(i===null||Lo.current!==null||i.memoizedState!==null)&&(ps=n)}function Zf(n){ce(Vt,Vt.current),ce(Fn,n),ps===null&&(ps=n)}function wy(n){n.tag===22?(ce(Vt,Vt.current),ce(Fn,n),ps===null&&(ps=n)):Xi()}function Xi(){ce(Vt,Vt.current),ce(Fn,Fn.current)}function Jn(n){Y(Fn),ps===n&&(ps=null),Y(Vt)}var Vt=z(0);function Rc(n){for(var i=n;i!==null;){if(i.tag===13){var r=i.memoizedState;if(r!==null&&(r=r.dehydrated,r===null||Xp(r)||Fp(r)))return i}else if(i.tag===19&&(i.memoizedProps.revealOrder==="forwards"||i.memoizedProps.revealOrder==="backwards"||i.memoizedProps.revealOrder==="unstable_legacy-backwards"||i.memoizedProps.revealOrder==="together")){if((i.flags&128)!==0)return i}else if(i.child!==null){i.child.return=i,i=i.child;continue}if(i===n)break;for(;i.sibling===null;){if(i.return===null||i.return===n)return null;i=i.return}i.sibling.return=i.return,i=i.sibling}return null}var di=0,Pe=null,mt=null,en=null,Lc=!1,qo=!1,Pa=!1,qc=0,el=0,Uo=null,bj=0;function It(){throw Error(a(321))}function Pf(n,i){if(i===null)return!1;for(var r=0;r<i.length&&r<n.length;r++)if(!Gn(n[r],i[r]))return!1;return!0}function Yf(n,i,r,u,h,g){return di=g,Pe=i,i.memoizedState=null,i.updateQueue=null,i.lanes=0,D.H=n===null||n.memoizedState===null?a_:op,Pa=!1,g=r(u,h),Pa=!1,qo&&(g=ky(i,r,u,h)),Sy(n),g}function Sy(n){D.H=sl;var i=mt!==null&&mt.next!==null;if(di=0,en=mt=Pe=null,Lc=!1,el=0,Uo=null,i)throw Error(a(300));n===null||tn||(n=n.dependencies,n!==null&&Tc(n)&&(tn=!0))}function ky(n,i,r,u){Pe=n;var h=0;do{if(qo&&(Uo=null),el=0,qo=!1,25<=h)throw Error(a(301));if(h+=1,en=mt=null,n.updateQueue!=null){var g=n.updateQueue;g.lastEffect=null,g.events=null,g.stores=null,g.memoCache!=null&&(g.memoCache.index=0)}D.H=o_,g=i(r,u)}while(qo);return g}function vj(){var n=D.H,i=n.useState()[0];return i=typeof i.then=="function"?tl(i):i,n=n.useState()[0],(mt!==null?mt.memoizedState:null)!==n&&(Pe.flags|=1024),i}function Vf(){var n=qc!==0;return qc=0,n}function Kf(n,i,r){i.updateQueue=n.updateQueue,i.flags&=-2053,n.lanes&=~r}function Gf(n){if(Lc){for(n=n.memoizedState;n!==null;){var i=n.queue;i!==null&&(i.pending=null),n=n.next}Lc=!1}di=0,en=mt=Pe=null,qo=!1,el=qc=0,Uo=null}function An(){var n={memoizedState:null,baseState:null,baseQueue:null,queue:null,next:null};return en===null?Pe.memoizedState=en=n:en=en.next=n,en}function Kt(){if(mt===null){var n=Pe.alternate;n=n!==null?n.memoizedState:null}else n=mt.next;var i=en===null?Pe.memoizedState:en.next;if(i!==null)en=i,mt=n;else{if(n===null)throw Pe.alternate===null?Error(a(467)):Error(a(310));mt=n,n={memoizedState:mt.memoizedState,baseState:mt.baseState,baseQueue:mt.baseQueue,queue:mt.queue,next:null},en===null?Pe.memoizedState=en=n:en=en.next=n}return en}function Uc(){return{lastEffect:null,events:null,stores:null,memoCache:null}}function tl(n){var i=el;return el+=1,Uo===null&&(Uo=[]),n=my(Uo,n,i),i=Pe,(en===null?i.memoizedState:en.next)===null&&(i=i.alternate,D.H=i===null||i.memoizedState===null?a_:op),n}function Bc(n){if(n!==null&&typeof n=="object"){if(typeof n.then=="function")return tl(n);if(n.$$typeof===N)return bn(n)}throw Error(a(438,String(n)))}function Xf(n){var i=null,r=Pe.updateQueue;if(r!==null&&(i=r.memoCache),i==null){var u=Pe.alternate;u!==null&&(u=u.updateQueue,u!==null&&(u=u.memoCache,u!=null&&(i={data:u.data.map(function(h){return h.slice()}),index:0})))}if(i==null&&(i={data:[],index:0}),r===null&&(r=Uc(),Pe.updateQueue=r),r.memoCache=i,r=i.data[i.index],r===void 0)for(r=i.data[i.index]=Array(n),u=0;u<n;u++)r[u]=oe;return i.index++,r}function fi(n,i){return typeof i=="function"?i(n):i}function Hc(n){var i=Kt();return Ff(i,mt,n)}function Ff(n,i,r){var u=n.queue;if(u===null)throw Error(a(311));u.lastRenderedReducer=r;var h=n.baseQueue,g=u.pending;if(g!==null){if(h!==null){var x=h.next;h.next=g.next,g.next=x}i.baseQueue=h=g,u.pending=null}if(g=n.baseState,h===null)n.memoizedState=g;else{i=h.next;var M=x=null,H=null,se=i,fe=!1;do{var ye=se.lane&-536870913;if(ye!==se.lane?(Qe&ye)===ye:(di&ye)===ye){var ie=se.revertLane;if(ie===0)H!==null&&(H=H.next={lane:0,revertLane:0,gesture:null,action:se.action,hasEagerState:se.hasEagerState,eagerState:se.eagerState,next:null}),ye===zo&&(fe=!0);else if((di&ie)===ie){se=se.next,ie===zo&&(fe=!0);continue}else ye={lane:0,revertLane:se.revertLane,gesture:null,action:se.action,hasEagerState:se.hasEagerState,eagerState:se.eagerState,next:null},H===null?(M=H=ye,x=g):H=H.next=ye,Pe.lanes|=ie,Qi|=ie;ye=se.action,Pa&&r(g,ye),g=se.hasEagerState?se.eagerState:r(g,ye)}else ie={lane:ye,revertLane:se.revertLane,gesture:se.gesture,action:se.action,hasEagerState:se.hasEagerState,eagerState:se.eagerState,next:null},H===null?(M=H=ie,x=g):H=H.next=ie,Pe.lanes|=ye,Qi|=ye;se=se.next}while(se!==null&&se!==i);if(H===null?x=g:H.next=M,!Gn(g,n.memoizedState)&&(tn=!0,fe&&(r=Do,r!==null)))throw r;n.memoizedState=g,n.baseState=x,n.baseQueue=H,u.lastRenderedState=g}return h===null&&(u.lanes=0),[n.memoizedState,u.dispatch]}function Jf(n){var i=Kt(),r=i.queue;if(r===null)throw Error(a(311));r.lastRenderedReducer=n;var u=r.dispatch,h=r.pending,g=i.memoizedState;if(h!==null){r.pending=null;var x=h=h.next;do g=n(g,x.action),x=x.next;while(x!==h);Gn(g,i.memoizedState)||(tn=!0),i.memoizedState=g,i.baseQueue===null&&(i.baseState=g),r.lastRenderedState=g}return[g,u]}function jy(n,i,r){var u=Pe,h=Kt(),g=et;if(g){if(r===void 0)throw Error(a(407));r=r()}else r=i();var x=!Gn((mt||h).memoizedState,r);if(x&&(h.memoizedState=r,tn=!0),h=h.queue,ep(Ty.bind(null,u,h,n),[n]),h.getSnapshot!==i||x||en!==null&&en.memoizedState.tag&1){if(u.flags|=2048,Bo(9,{destroy:void 0},Ey.bind(null,u,h,r,i),null),_t===null)throw Error(a(349));g||(di&127)!==0||Ny(u,i,r)}return r}function Ny(n,i,r){n.flags|=16384,n={getSnapshot:i,value:r},i=Pe.updateQueue,i===null?(i=Uc(),Pe.updateQueue=i,i.stores=[n]):(r=i.stores,r===null?i.stores=[n]:r.push(n))}function Ey(n,i,r,u){i.value=r,i.getSnapshot=u,Cy(i)&&Ay(n)}function Ty(n,i,r){return r(function(){Cy(i)&&Ay(n)})}function Cy(n){var i=n.getSnapshot;n=n.value;try{var r=i();return!Gn(n,r)}catch{return!0}}function Ay(n){var i=$a(n,2);i!==null&&Hn(i,n,2)}function Qf(n){var i=An();if(typeof n=="function"){var r=n;if(n=r(),Pa){Be(!0);try{r()}finally{Be(!1)}}}return i.memoizedState=i.baseState=n,i.queue={pending:null,lanes:0,dispatch:null,lastRenderedReducer:fi,lastRenderedState:n},i}function Oy(n,i,r,u){return n.baseState=r,Ff(n,mt,typeof u=="function"?u:fi)}function xj(n,i,r,u,h){if(Pc(n))throw Error(a(485));if(n=i.action,n!==null){var g={payload:h,action:n,next:null,isTransition:!0,status:"pending",value:null,reason:null,listeners:[],then:function(x){g.listeners.push(x)}};D.T!==null?r(!0):g.isTransition=!1,u(g),r=i.pending,r===null?(g.next=i.pending=g,My(i,g)):(g.next=r.next,i.pending=r.next=g)}}function My(n,i){var r=i.action,u=i.payload,h=n.state;if(i.isTransition){var g=D.T,x={};D.T=x;try{var M=r(h,u),H=D.S;H!==null&&H(x,M),zy(n,i,M)}catch(se){Wf(n,i,se)}finally{g!==null&&x.types!==null&&(g.types=x.types),D.T=g}}else try{g=r(h,u),zy(n,i,g)}catch(se){Wf(n,i,se)}}function zy(n,i,r){r!==null&&typeof r=="object"&&typeof r.then=="function"?r.then(function(u){Dy(n,i,u)},function(u){return Wf(n,i,u)}):Dy(n,i,r)}function Dy(n,i,r){i.status="fulfilled",i.value=r,$y(i),n.state=r,i=n.pending,i!==null&&(r=i.next,r===i?n.pending=null:(r=r.next,i.next=r,My(n,r)))}function Wf(n,i,r){var u=n.pending;if(n.pending=null,u!==null){u=u.next;do i.status="rejected",i.reason=r,$y(i),i=i.next;while(i!==u)}n.action=null}function $y(n){n=n.listeners;for(var i=0;i<n.length;i++)(0,n[i])()}function Ry(n,i){return i}function Ly(n,i){if(et){var r=_t.formState;if(r!==null){e:{var u=Pe;if(et){if(jt){t:{for(var h=jt,g=fs;h.nodeType!==8;){if(!g){h=null;break t}if(h=ms(h.nextSibling),h===null){h=null;break t}}g=h.data,h=g==="F!"||g==="F"?h:null}if(h){jt=ms(h.nextSibling),u=h.data==="F!";break e}}Zi(u)}u=!1}u&&(i=r[0])}}return r=An(),r.memoizedState=r.baseState=i,u={pending:null,lanes:0,dispatch:null,lastRenderedReducer:Ry,lastRenderedState:i},r.queue=u,r=n_.bind(null,Pe,u),u.dispatch=r,u=Qf(!1),g=ap.bind(null,Pe,!1,u.queue),u=An(),h={state:i,dispatch:null,action:n,pending:null},u.queue=h,r=xj.bind(null,Pe,h,g,r),h.dispatch=r,u.memoizedState=n,[i,r,!1]}function qy(n){var i=Kt();return Uy(i,mt,n)}function Uy(n,i,r){if(i=Ff(n,i,Ry)[0],n=Hc(fi)[0],typeof i=="object"&&i!==null&&typeof i.then=="function")try{var u=tl(i)}catch(x){throw x===$o?Oc:x}else u=i;i=Kt();var h=i.queue,g=h.dispatch;return r!==i.memoizedState&&(Pe.flags|=2048,Bo(9,{destroy:void 0},wj.bind(null,h,r),null)),[u,g,n]}function wj(n,i){n.action=i}function By(n){var i=Kt(),r=mt;if(r!==null)return Uy(i,r,n);Kt(),i=i.memoizedState,r=Kt();var u=r.queue.dispatch;return r.memoizedState=n,[i,u,!1]}function Bo(n,i,r,u){return n={tag:n,create:r,deps:u,inst:i,next:null},i=Pe.updateQueue,i===null&&(i=Uc(),Pe.updateQueue=i),r=i.lastEffect,r===null?i.lastEffect=n.next=n:(u=r.next,r.next=n,n.next=u,i.lastEffect=n),n}function Hy(){return Kt().memoizedState}function Ic(n,i,r,u){var h=An();Pe.flags|=n,h.memoizedState=Bo(1|i,{destroy:void 0},r,u===void 0?null:u)}function Zc(n,i,r,u){var h=Kt();u=u===void 0?null:u;var g=h.memoizedState.inst;mt!==null&&u!==null&&Pf(u,mt.memoizedState.deps)?h.memoizedState=Bo(i,g,r,u):(Pe.flags|=n,h.memoizedState=Bo(1|i,g,r,u))}function Iy(n,i){Ic(8390656,8,n,i)}function ep(n,i){Zc(2048,8,n,i)}function Sj(n){Pe.flags|=4;var i=Pe.updateQueue;if(i===null)i=Uc(),Pe.updateQueue=i,i.events=[n];else{var r=i.events;r===null?i.events=[n]:r.push(n)}}function Zy(n){var i=Kt().memoizedState;return Sj({ref:i,nextImpl:n}),function(){if((ut&2)!==0)throw Error(a(440));return i.impl.apply(void 0,arguments)}}function Py(n,i){return Zc(4,2,n,i)}function Yy(n,i){return Zc(4,4,n,i)}function Vy(n,i){if(typeof i=="function"){n=n();var r=i(n);return function(){typeof r=="function"?r():i(null)}}if(i!=null)return n=n(),i.current=n,function(){i.current=null}}function Ky(n,i,r){r=r!=null?r.concat([n]):null,Zc(4,4,Vy.bind(null,i,n),r)}function tp(){}function Gy(n,i){var r=Kt();i=i===void 0?null:i;var u=r.memoizedState;return i!==null&&Pf(i,u[1])?u[0]:(r.memoizedState=[n,i],n)}function Xy(n,i){var r=Kt();i=i===void 0?null:i;var u=r.memoizedState;if(i!==null&&Pf(i,u[1]))return u[0];if(u=n(),Pa){Be(!0);try{n()}finally{Be(!1)}}return r.memoizedState=[u,i],u}function np(n,i,r){return r===void 0||(di&1073741824)!==0&&(Qe&261930)===0?n.memoizedState=i:(n.memoizedState=r,n=F_(),Pe.lanes|=n,Qi|=n,r)}function Fy(n,i,r,u){return Gn(r,i)?r:Lo.current!==null?(n=np(n,r,u),Gn(n,i)||(tn=!0),n):(di&42)===0||(di&1073741824)!==0&&(Qe&261930)===0?(tn=!0,n.memoizedState=r):(n=F_(),Pe.lanes|=n,Qi|=n,i)}function Jy(n,i,r,u,h){var g=U.p;U.p=g!==0&&8>g?g:8;var x=D.T,M={};D.T=M,ap(n,!1,i,r);try{var H=h(),se=D.S;if(se!==null&&se(M,H),H!==null&&typeof H=="object"&&typeof H.then=="function"){var fe=_j(H,u);nl(n,i,fe,es(n))}else nl(n,i,u,es(n))}catch(ye){nl(n,i,{then:function(){},status:"rejected",reason:ye},es())}finally{U.p=g,x!==null&&M.types!==null&&(x.types=M.types),D.T=x}}function kj(){}function sp(n,i,r,u){if(n.tag!==5)throw Error(a(476));var h=Qy(n).queue;Jy(n,h,i,B,r===null?kj:function(){return Wy(n),r(u)})}function Qy(n){var i=n.memoizedState;if(i!==null)return i;i={memoizedState:B,baseState:B,baseQueue:null,queue:{pending:null,lanes:0,dispatch:null,lastRenderedReducer:fi,lastRenderedState:B},next:null};var r={};return i.next={memoizedState:r,baseState:r,baseQueue:null,queue:{pending:null,lanes:0,dispatch:null,lastRenderedReducer:fi,lastRenderedState:r},next:null},n.memoizedState=i,n=n.alternate,n!==null&&(n.memoizedState=i),i}function Wy(n){var i=Qy(n);i.next===null&&(i=n.alternate.memoizedState),nl(n,i.next.queue,{},es())}function ip(){return bn(bl)}function e_(){return Kt().memoizedState}function t_(){return Kt().memoizedState}function jj(n){for(var i=n.return;i!==null;){switch(i.tag){case 24:case 3:var r=es();n=Vi(r);var u=Ki(i,n,r);u!==null&&(Hn(u,i,r),Jr(u,i,r)),i={cache:zf()},n.payload=i;return}i=i.return}}function Nj(n,i,r){var u=es();r={lane:u,revertLane:0,gesture:null,action:r,hasEagerState:!1,eagerState:null,next:null},Pc(n)?s_(i,r):(r=wf(n,i,r,u),r!==null&&(Hn(r,n,u),i_(r,i,u)))}function n_(n,i,r){var u=es();nl(n,i,r,u)}function nl(n,i,r,u){var h={lane:u,revertLane:0,gesture:null,action:r,hasEagerState:!1,eagerState:null,next:null};if(Pc(n))s_(i,h);else{var g=n.alternate;if(n.lanes===0&&(g===null||g.lanes===0)&&(g=i.lastRenderedReducer,g!==null))try{var x=i.lastRenderedState,M=g(x,r);if(h.hasEagerState=!0,h.eagerState=M,Gn(M,x))return kc(n,i,h,0),_t===null&&Sc(),!1}catch{}if(r=wf(n,i,h,u),r!==null)return Hn(r,n,u),i_(r,i,u),!0}return!1}function ap(n,i,r,u){if(u={lane:2,revertLane:Lp(),gesture:null,action:u,hasEagerState:!1,eagerState:null,next:null},Pc(n)){if(i)throw Error(a(479))}else i=wf(n,r,u,2),i!==null&&Hn(i,n,2)}function Pc(n){var i=n.alternate;return n===Pe||i!==null&&i===Pe}function s_(n,i){qo=Lc=!0;var r=n.pending;r===null?i.next=i:(i.next=r.next,r.next=i),n.pending=i}function i_(n,i,r){if((r&4194048)!==0){var u=i.lanes;u&=n.pendingLanes,r|=u,i.lanes=r,xa(n,r)}}var sl={readContext:bn,use:Bc,useCallback:It,useContext:It,useEffect:It,useImperativeHandle:It,useLayoutEffect:It,useInsertionEffect:It,useMemo:It,useReducer:It,useRef:It,useState:It,useDebugValue:It,useDeferredValue:It,useTransition:It,useSyncExternalStore:It,useId:It,useHostTransitionStatus:It,useFormState:It,useActionState:It,useOptimistic:It,useMemoCache:It,useCacheRefresh:It};sl.useEffectEvent=It;var a_={readContext:bn,use:Bc,useCallback:function(n,i){return An().memoizedState=[n,i===void 0?null:i],n},useContext:bn,useEffect:Iy,useImperativeHandle:function(n,i,r){r=r!=null?r.concat([n]):null,Ic(4194308,4,Vy.bind(null,i,n),r)},useLayoutEffect:function(n,i){return Ic(4194308,4,n,i)},useInsertionEffect:function(n,i){Ic(4,2,n,i)},useMemo:function(n,i){var r=An();i=i===void 0?null:i;var u=n();if(Pa){Be(!0);try{n()}finally{Be(!1)}}return r.memoizedState=[u,i],u},useReducer:function(n,i,r){var u=An();if(r!==void 0){var h=r(i);if(Pa){Be(!0);try{r(i)}finally{Be(!1)}}}else h=i;return u.memoizedState=u.baseState=h,n={pending:null,lanes:0,dispatch:null,lastRenderedReducer:n,lastRenderedState:h},u.queue=n,n=n.dispatch=Nj.bind(null,Pe,n),[u.memoizedState,n]},useRef:function(n){var i=An();return n={current:n},i.memoizedState=n},useState:function(n){n=Qf(n);var i=n.queue,r=n_.bind(null,Pe,i);return i.dispatch=r,[n.memoizedState,r]},useDebugValue:tp,useDeferredValue:function(n,i){var r=An();return np(r,n,i)},useTransition:function(){var n=Qf(!1);return n=Jy.bind(null,Pe,n.queue,!0,!1),An().memoizedState=n,[!1,n]},useSyncExternalStore:function(n,i,r){var u=Pe,h=An();if(et){if(r===void 0)throw Error(a(407));r=r()}else{if(r=i(),_t===null)throw Error(a(349));(Qe&127)!==0||Ny(u,i,r)}h.memoizedState=r;var g={value:r,getSnapshot:i};return h.queue=g,Iy(Ty.bind(null,u,g,n),[n]),u.flags|=2048,Bo(9,{destroy:void 0},Ey.bind(null,u,g,r,i),null),r},useId:function(){var n=An(),i=_t.identifierPrefix;if(et){var r=Ps,u=Zs;r=(u&~(1<<32-Mt(u)-1)).toString(32)+r,i="_"+i+"R_"+r,r=qc++,0<r&&(i+="H"+r.toString(32)),i+="_"}else r=bj++,i="_"+i+"r_"+r.toString(32)+"_";return n.memoizedState=i},useHostTransitionStatus:ip,useFormState:Ly,useActionState:Ly,useOptimistic:function(n){var i=An();i.memoizedState=i.baseState=n;var r={pending:null,lanes:0,dispatch:null,lastRenderedReducer:null,lastRenderedState:null};return i.queue=r,i=ap.bind(null,Pe,!0,r),r.dispatch=i,[n,i]},useMemoCache:Xf,useCacheRefresh:function(){return An().memoizedState=jj.bind(null,Pe)},useEffectEvent:function(n){var i=An(),r={impl:n};return i.memoizedState=r,function(){if((ut&2)!==0)throw Error(a(440));return r.impl.apply(void 0,arguments)}}},op={readContext:bn,use:Bc,useCallback:Gy,useContext:bn,useEffect:ep,useImperativeHandle:Ky,useInsertionEffect:Py,useLayoutEffect:Yy,useMemo:Xy,useReducer:Hc,useRef:Hy,useState:function(){return Hc(fi)},useDebugValue:tp,useDeferredValue:function(n,i){var r=Kt();return Fy(r,mt.memoizedState,n,i)},useTransition:function(){var n=Hc(fi)[0],i=Kt().memoizedState;return[typeof n=="boolean"?n:tl(n),i]},useSyncExternalStore:jy,useId:e_,useHostTransitionStatus:ip,useFormState:qy,useActionState:qy,useOptimistic:function(n,i){var r=Kt();return Oy(r,mt,n,i)},useMemoCache:Xf,useCacheRefresh:t_};op.useEffectEvent=Zy;var o_={readContext:bn,use:Bc,useCallback:Gy,useContext:bn,useEffect:ep,useImperativeHandle:Ky,useInsertionEffect:Py,useLayoutEffect:Yy,useMemo:Xy,useReducer:Jf,useRef:Hy,useState:function(){return Jf(fi)},useDebugValue:tp,useDeferredValue:function(n,i){var r=Kt();return mt===null?np(r,n,i):Fy(r,mt.memoizedState,n,i)},useTransition:function(){var n=Jf(fi)[0],i=Kt().memoizedState;return[typeof n=="boolean"?n:tl(n),i]},useSyncExternalStore:jy,useId:e_,useHostTransitionStatus:ip,useFormState:By,useActionState:By,useOptimistic:function(n,i){var r=Kt();return mt!==null?Oy(r,mt,n,i):(r.baseState=n,[n,r.queue.dispatch])},useMemoCache:Xf,useCacheRefresh:t_};o_.useEffectEvent=Zy;function rp(n,i,r,u){i=n.memoizedState,r=r(u,i),r=r==null?i:y({},i,r),n.memoizedState=r,n.lanes===0&&(n.updateQueue.baseState=r)}var lp={enqueueSetState:function(n,i,r){n=n._reactInternals;var u=es(),h=Vi(u);h.payload=i,r!=null&&(h.callback=r),i=Ki(n,h,u),i!==null&&(Hn(i,n,u),Jr(i,n,u))},enqueueReplaceState:function(n,i,r){n=n._reactInternals;var u=es(),h=Vi(u);h.tag=1,h.payload=i,r!=null&&(h.callback=r),i=Ki(n,h,u),i!==null&&(Hn(i,n,u),Jr(i,n,u))},enqueueForceUpdate:function(n,i){n=n._reactInternals;var r=es(),u=Vi(r);u.tag=2,i!=null&&(u.callback=i),i=Ki(n,u,r),i!==null&&(Hn(i,n,r),Jr(i,n,r))}};function r_(n,i,r,u,h,g,x){return n=n.stateNode,typeof n.shouldComponentUpdate=="function"?n.shouldComponentUpdate(u,g,x):i.prototype&&i.prototype.isPureReactComponent?!Zr(r,u)||!Zr(h,g):!0}function l_(n,i,r,u){n=i.state,typeof i.componentWillReceiveProps=="function"&&i.componentWillReceiveProps(r,u),typeof i.UNSAFE_componentWillReceiveProps=="function"&&i.UNSAFE_componentWillReceiveProps(r,u),i.state!==n&&lp.enqueueReplaceState(i,i.state,null)}function Ya(n,i){var r=i;if("ref"in i){r={};for(var u in i)u!=="ref"&&(r[u]=i[u])}if(n=n.defaultProps){r===i&&(r=y({},r));for(var h in n)r[h]===void 0&&(r[h]=n[h])}return r}function c_(n){wc(n)}function u_(n){console.error(n)}function d_(n){wc(n)}function Yc(n,i){try{var r=n.onUncaughtError;r(i.value,{componentStack:i.stack})}catch(u){setTimeout(function(){throw u})}}function f_(n,i,r){try{var u=n.onCaughtError;u(r.value,{componentStack:r.stack,errorBoundary:i.tag===1?i.stateNode:null})}catch(h){setTimeout(function(){throw h})}}function cp(n,i,r){return r=Vi(r),r.tag=3,r.payload={element:null},r.callback=function(){Yc(n,i)},r}function p_(n){return n=Vi(n),n.tag=3,n}function m_(n,i,r,u){var h=r.type.getDerivedStateFromError;if(typeof h=="function"){var g=u.value;n.payload=function(){return h(g)},n.callback=function(){f_(i,r,u)}}var x=r.stateNode;x!==null&&typeof x.componentDidCatch=="function"&&(n.callback=function(){f_(i,r,u),typeof h!="function"&&(Wi===null?Wi=new Set([this]):Wi.add(this));var M=u.stack;this.componentDidCatch(u.value,{componentStack:M!==null?M:""})})}function Ej(n,i,r,u,h){if(r.flags|=32768,u!==null&&typeof u=="object"&&typeof u.then=="function"){if(i=r.alternate,i!==null&&Mo(i,r,h,!0),r=Fn.current,r!==null){switch(r.tag){case 31:case 13:return ps===null?su():r.alternate===null&&Zt===0&&(Zt=3),r.flags&=-257,r.flags|=65536,r.lanes=h,u===Mc?r.flags|=16384:(i=r.updateQueue,i===null?r.updateQueue=new Set([u]):i.add(u),Dp(n,u,h)),!1;case 22:return r.flags|=65536,u===Mc?r.flags|=16384:(i=r.updateQueue,i===null?(i={transitions:null,markerInstances:null,retryQueue:new Set([u])},r.updateQueue=i):(r=i.retryQueue,r===null?i.retryQueue=new Set([u]):r.add(u)),Dp(n,u,h)),!1}throw Error(a(435,r.tag))}return Dp(n,u,h),su(),!1}if(et)return i=Fn.current,i!==null?((i.flags&65536)===0&&(i.flags|=256),i.flags|=65536,i.lanes=h,u!==Tf&&(n=Error(a(422),{cause:u}),Vr(cs(n,r)))):(u!==Tf&&(i=Error(a(423),{cause:u}),Vr(cs(i,r))),n=n.current.alternate,n.flags|=65536,h&=-h,n.lanes|=h,u=cs(u,r),h=cp(n.stateNode,u,h),Uf(n,h),Zt!==4&&(Zt=2)),!1;var g=Error(a(520),{cause:u});if(g=cs(g,r),dl===null?dl=[g]:dl.push(g),Zt!==4&&(Zt=2),i===null)return!0;u=cs(u,r),r=i;do{switch(r.tag){case 3:return r.flags|=65536,n=h&-h,r.lanes|=n,n=cp(r.stateNode,u,n),Uf(r,n),!1;case 1:if(i=r.type,g=r.stateNode,(r.flags&128)===0&&(typeof i.getDerivedStateFromError=="function"||g!==null&&typeof g.componentDidCatch=="function"&&(Wi===null||!Wi.has(g))))return r.flags|=65536,h&=-h,r.lanes|=h,h=p_(h),m_(h,n,r,u),Uf(r,h),!1}r=r.return}while(r!==null);return!1}var up=Error(a(461)),tn=!1;function vn(n,i,r,u){i.child=n===null?_y(i,null,r,u):Za(i,n.child,r,u)}function h_(n,i,r,u,h){r=r.render;var g=i.ref;if("ref"in u){var x={};for(var M in u)M!=="ref"&&(x[M]=u[M])}else x=u;return Ua(i),u=Yf(n,i,r,x,g,h),M=Vf(),n!==null&&!tn?(Kf(n,i,h),pi(n,i,h)):(et&&M&&Nf(i),i.flags|=1,vn(n,i,u,h),i.child)}function g_(n,i,r,u,h){if(n===null){var g=r.type;return typeof g=="function"&&!Sf(g)&&g.defaultProps===void 0&&r.compare===null?(i.tag=15,i.type=g,y_(n,i,g,u,h)):(n=Nc(r.type,null,u,i,i.mode,h),n.ref=i.ref,n.return=i,i.child=n)}if(g=n.child,!_p(n,h)){var x=g.memoizedProps;if(r=r.compare,r=r!==null?r:Zr,r(x,u)&&n.ref===i.ref)return pi(n,i,h)}return i.flags|=1,n=ri(g,u),n.ref=i.ref,n.return=i,i.child=n}function y_(n,i,r,u,h){if(n!==null){var g=n.memoizedProps;if(Zr(g,u)&&n.ref===i.ref)if(tn=!1,i.pendingProps=u=g,_p(n,h))(n.flags&131072)!==0&&(tn=!0);else return i.lanes=n.lanes,pi(n,i,h)}return dp(n,i,r,u,h)}function __(n,i,r,u){var h=u.children,g=n!==null?n.memoizedState:null;if(n===null&&i.stateNode===null&&(i.stateNode={_visibility:1,_pendingMarkers:null,_retryCache:null,_transitions:null}),u.mode==="hidden"){if((i.flags&128)!==0){if(g=g!==null?g.baseLanes|r:r,n!==null){for(u=i.child=n.child,h=0;u!==null;)h=h|u.lanes|u.childLanes,u=u.sibling;u=h&~g}else u=0,i.child=null;return b_(n,i,g,r,u)}if((r&536870912)!==0)i.memoizedState={baseLanes:0,cachePool:null},n!==null&&Ac(i,g!==null?g.cachePool:null),g!==null?xy(i,g):Hf(),wy(i);else return u=i.lanes=536870912,b_(n,i,g!==null?g.baseLanes|r:r,r,u)}else g!==null?(Ac(i,g.cachePool),xy(i,g),Xi(),i.memoizedState=null):(n!==null&&Ac(i,null),Hf(),Xi());return vn(n,i,h,r),i.child}function il(n,i){return n!==null&&n.tag===22||i.stateNode!==null||(i.stateNode={_visibility:1,_pendingMarkers:null,_retryCache:null,_transitions:null}),i.sibling}function b_(n,i,r,u,h){var g=$f();return g=g===null?null:{parent:Wt._currentValue,pool:g},i.memoizedState={baseLanes:r,cachePool:g},n!==null&&Ac(i,null),Hf(),wy(i),n!==null&&Mo(n,i,u,!0),i.childLanes=h,null}function Vc(n,i){return i=Gc({mode:i.mode,children:i.children},n.mode),i.ref=n.ref,n.child=i,i.return=n,i}function v_(n,i,r){return Za(i,n.child,null,r),n=Vc(i,i.pendingProps),n.flags|=2,Jn(i),i.memoizedState=null,n}function Tj(n,i,r){var u=i.pendingProps,h=(i.flags&128)!==0;if(i.flags&=-129,n===null){if(et){if(u.mode==="hidden")return n=Vc(i,u),i.lanes=536870912,il(null,n);if(Zf(i),(n=jt)?(n=Mb(n,fs),n=n!==null&&n.data==="&"?n:null,n!==null&&(i.memoizedState={dehydrated:n,treeContext:Hi!==null?{id:Zs,overflow:Ps}:null,retryLane:536870912,hydrationErrors:null},r=sy(n),r.return=i,i.child=r,_n=i,jt=null)):n=null,n===null)throw Zi(i);return i.lanes=536870912,null}return Vc(i,u)}var g=n.memoizedState;if(g!==null){var x=g.dehydrated;if(Zf(i),h)if(i.flags&256)i.flags&=-257,i=v_(n,i,r);else if(i.memoizedState!==null)i.child=n.child,i.flags|=128,i=null;else throw Error(a(558));else if(tn||Mo(n,i,r,!1),h=(r&n.childLanes)!==0,tn||h){if(u=_t,u!==null&&(x=ti(u,r),x!==0&&x!==g.retryLane))throw g.retryLane=x,$a(n,x),Hn(u,n,x),up;su(),i=v_(n,i,r)}else n=g.treeContext,jt=ms(x.nextSibling),_n=i,et=!0,Ii=null,fs=!1,n!==null&&oy(i,n),i=Vc(i,u),i.flags|=4096;return i}return n=ri(n.child,{mode:u.mode,children:u.children}),n.ref=i.ref,i.child=n,n.return=i,n}function Kc(n,i){var r=i.ref;if(r===null)n!==null&&n.ref!==null&&(i.flags|=4194816);else{if(typeof r!="function"&&typeof r!="object")throw Error(a(284));(n===null||n.ref!==r)&&(i.flags|=4194816)}}function dp(n,i,r,u,h){return Ua(i),r=Yf(n,i,r,u,void 0,h),u=Vf(),n!==null&&!tn?(Kf(n,i,h),pi(n,i,h)):(et&&u&&Nf(i),i.flags|=1,vn(n,i,r,h),i.child)}function x_(n,i,r,u,h,g){return Ua(i),i.updateQueue=null,r=ky(i,u,r,h),Sy(n),u=Vf(),n!==null&&!tn?(Kf(n,i,g),pi(n,i,g)):(et&&u&&Nf(i),i.flags|=1,vn(n,i,r,g),i.child)}function w_(n,i,r,u,h){if(Ua(i),i.stateNode===null){var g=To,x=r.contextType;typeof x=="object"&&x!==null&&(g=bn(x)),g=new r(u,g),i.memoizedState=g.state!==null&&g.state!==void 0?g.state:null,g.updater=lp,i.stateNode=g,g._reactInternals=i,g=i.stateNode,g.props=u,g.state=i.memoizedState,g.refs={},Lf(i),x=r.contextType,g.context=typeof x=="object"&&x!==null?bn(x):To,g.state=i.memoizedState,x=r.getDerivedStateFromProps,typeof x=="function"&&(rp(i,r,x,u),g.state=i.memoizedState),typeof r.getDerivedStateFromProps=="function"||typeof g.getSnapshotBeforeUpdate=="function"||typeof g.UNSAFE_componentWillMount!="function"&&typeof g.componentWillMount!="function"||(x=g.state,typeof g.componentWillMount=="function"&&g.componentWillMount(),typeof g.UNSAFE_componentWillMount=="function"&&g.UNSAFE_componentWillMount(),x!==g.state&&lp.enqueueReplaceState(g,g.state,null),Wr(i,u,g,h),Qr(),g.state=i.memoizedState),typeof g.componentDidMount=="function"&&(i.flags|=4194308),u=!0}else if(n===null){g=i.stateNode;var M=i.memoizedProps,H=Ya(r,M);g.props=H;var se=g.context,fe=r.contextType;x=To,typeof fe=="object"&&fe!==null&&(x=bn(fe));var ye=r.getDerivedStateFromProps;fe=typeof ye=="function"||typeof g.getSnapshotBeforeUpdate=="function",M=i.pendingProps!==M,fe||typeof g.UNSAFE_componentWillReceiveProps!="function"&&typeof g.componentWillReceiveProps!="function"||(M||se!==x)&&l_(i,g,u,x),Yi=!1;var ie=i.memoizedState;g.state=ie,Wr(i,u,g,h),Qr(),se=i.memoizedState,M||ie!==se||Yi?(typeof ye=="function"&&(rp(i,r,ye,u),se=i.memoizedState),(H=Yi||r_(i,r,H,u,ie,se,x))?(fe||typeof g.UNSAFE_componentWillMount!="function"&&typeof g.componentWillMount!="function"||(typeof g.componentWillMount=="function"&&g.componentWillMount(),typeof g.UNSAFE_componentWillMount=="function"&&g.UNSAFE_componentWillMount()),typeof g.componentDidMount=="function"&&(i.flags|=4194308)):(typeof g.componentDidMount=="function"&&(i.flags|=4194308),i.memoizedProps=u,i.memoizedState=se),g.props=u,g.state=se,g.context=x,u=H):(typeof g.componentDidMount=="function"&&(i.flags|=4194308),u=!1)}else{g=i.stateNode,qf(n,i),x=i.memoizedProps,fe=Ya(r,x),g.props=fe,ye=i.pendingProps,ie=g.context,se=r.contextType,H=To,typeof se=="object"&&se!==null&&(H=bn(se)),M=r.getDerivedStateFromProps,(se=typeof M=="function"||typeof g.getSnapshotBeforeUpdate=="function")||typeof g.UNSAFE_componentWillReceiveProps!="function"&&typeof g.componentWillReceiveProps!="function"||(x!==ye||ie!==H)&&l_(i,g,u,H),Yi=!1,ie=i.memoizedState,g.state=ie,Wr(i,u,g,h),Qr();var re=i.memoizedState;x!==ye||ie!==re||Yi||n!==null&&n.dependencies!==null&&Tc(n.dependencies)?(typeof M=="function"&&(rp(i,r,M,u),re=i.memoizedState),(fe=Yi||r_(i,r,fe,u,ie,re,H)||n!==null&&n.dependencies!==null&&Tc(n.dependencies))?(se||typeof g.UNSAFE_componentWillUpdate!="function"&&typeof g.componentWillUpdate!="function"||(typeof g.componentWillUpdate=="function"&&g.componentWillUpdate(u,re,H),typeof g.UNSAFE_componentWillUpdate=="function"&&g.UNSAFE_componentWillUpdate(u,re,H)),typeof g.componentDidUpdate=="function"&&(i.flags|=4),typeof g.getSnapshotBeforeUpdate=="function"&&(i.flags|=1024)):(typeof g.componentDidUpdate!="function"||x===n.memoizedProps&&ie===n.memoizedState||(i.flags|=4),typeof g.getSnapshotBeforeUpdate!="function"||x===n.memoizedProps&&ie===n.memoizedState||(i.flags|=1024),i.memoizedProps=u,i.memoizedState=re),g.props=u,g.state=re,g.context=H,u=fe):(typeof g.componentDidUpdate!="function"||x===n.memoizedProps&&ie===n.memoizedState||(i.flags|=4),typeof g.getSnapshotBeforeUpdate!="function"||x===n.memoizedProps&&ie===n.memoizedState||(i.flags|=1024),u=!1)}return g=u,Kc(n,i),u=(i.flags&128)!==0,g||u?(g=i.stateNode,r=u&&typeof r.getDerivedStateFromError!="function"?null:g.render(),i.flags|=1,n!==null&&u?(i.child=Za(i,n.child,null,h),i.child=Za(i,null,r,h)):vn(n,i,r,h),i.memoizedState=g.state,n=i.child):n=pi(n,i,h),n}function S_(n,i,r,u){return La(),i.flags|=256,vn(n,i,r,u),i.child}var fp={dehydrated:null,treeContext:null,retryLane:0,hydrationErrors:null};function pp(n){return{baseLanes:n,cachePool:fy()}}function mp(n,i,r){return n=n!==null?n.childLanes&~r:0,i&&(n|=Wn),n}function k_(n,i,r){var u=i.pendingProps,h=!1,g=(i.flags&128)!==0,x;if((x=g)||(x=n!==null&&n.memoizedState===null?!1:(Vt.current&2)!==0),x&&(h=!0,i.flags&=-129),x=(i.flags&32)!==0,i.flags&=-33,n===null){if(et){if(h?Gi(i):Xi(),(n=jt)?(n=Mb(n,fs),n=n!==null&&n.data!=="&"?n:null,n!==null&&(i.memoizedState={dehydrated:n,treeContext:Hi!==null?{id:Zs,overflow:Ps}:null,retryLane:536870912,hydrationErrors:null},r=sy(n),r.return=i,i.child=r,_n=i,jt=null)):n=null,n===null)throw Zi(i);return Fp(n)?i.lanes=32:i.lanes=536870912,null}var M=u.children;return u=u.fallback,h?(Xi(),h=i.mode,M=Gc({mode:"hidden",children:M},h),u=Ra(u,h,r,null),M.return=i,u.return=i,M.sibling=u,i.child=M,u=i.child,u.memoizedState=pp(r),u.childLanes=mp(n,x,r),i.memoizedState=fp,il(null,u)):(Gi(i),hp(i,M))}var H=n.memoizedState;if(H!==null&&(M=H.dehydrated,M!==null)){if(g)i.flags&256?(Gi(i),i.flags&=-257,i=gp(n,i,r)):i.memoizedState!==null?(Xi(),i.child=n.child,i.flags|=128,i=null):(Xi(),M=u.fallback,h=i.mode,u=Gc({mode:"visible",children:u.children},h),M=Ra(M,h,r,null),M.flags|=2,u.return=i,M.return=i,u.sibling=M,i.child=u,Za(i,n.child,null,r),u=i.child,u.memoizedState=pp(r),u.childLanes=mp(n,x,r),i.memoizedState=fp,i=il(null,u));else if(Gi(i),Fp(M)){if(x=M.nextSibling&&M.nextSibling.dataset,x)var se=x.dgst;x=se,u=Error(a(419)),u.stack="",u.digest=x,Vr({value:u,source:null,stack:null}),i=gp(n,i,r)}else if(tn||Mo(n,i,r,!1),x=(r&n.childLanes)!==0,tn||x){if(x=_t,x!==null&&(u=ti(x,r),u!==0&&u!==H.retryLane))throw H.retryLane=u,$a(n,u),Hn(x,n,u),up;Xp(M)||su(),i=gp(n,i,r)}else Xp(M)?(i.flags|=192,i.child=n.child,i=null):(n=H.treeContext,jt=ms(M.nextSibling),_n=i,et=!0,Ii=null,fs=!1,n!==null&&oy(i,n),i=hp(i,u.children),i.flags|=4096);return i}return h?(Xi(),M=u.fallback,h=i.mode,H=n.child,se=H.sibling,u=ri(H,{mode:"hidden",children:u.children}),u.subtreeFlags=H.subtreeFlags&65011712,se!==null?M=ri(se,M):(M=Ra(M,h,r,null),M.flags|=2),M.return=i,u.return=i,u.sibling=M,i.child=u,il(null,u),u=i.child,M=n.child.memoizedState,M===null?M=pp(r):(h=M.cachePool,h!==null?(H=Wt._currentValue,h=h.parent!==H?{parent:H,pool:H}:h):h=fy(),M={baseLanes:M.baseLanes|r,cachePool:h}),u.memoizedState=M,u.childLanes=mp(n,x,r),i.memoizedState=fp,il(n.child,u)):(Gi(i),r=n.child,n=r.sibling,r=ri(r,{mode:"visible",children:u.children}),r.return=i,r.sibling=null,n!==null&&(x=i.deletions,x===null?(i.deletions=[n],i.flags|=16):x.push(n)),i.child=r,i.memoizedState=null,r)}function hp(n,i){return i=Gc({mode:"visible",children:i},n.mode),i.return=n,n.child=i}function Gc(n,i){return n=Xn(22,n,null,i),n.lanes=0,n}function gp(n,i,r){return Za(i,n.child,null,r),n=hp(i,i.pendingProps.children),n.flags|=2,i.memoizedState=null,n}function j_(n,i,r){n.lanes|=i;var u=n.alternate;u!==null&&(u.lanes|=i),Of(n.return,i,r)}function yp(n,i,r,u,h,g){var x=n.memoizedState;x===null?n.memoizedState={isBackwards:i,rendering:null,renderingStartTime:0,last:u,tail:r,tailMode:h,treeForkCount:g}:(x.isBackwards=i,x.rendering=null,x.renderingStartTime=0,x.last=u,x.tail=r,x.tailMode=h,x.treeForkCount=g)}function N_(n,i,r){var u=i.pendingProps,h=u.revealOrder,g=u.tail;u=u.children;var x=Vt.current,M=(x&2)!==0;if(M?(x=x&1|2,i.flags|=128):x&=1,ce(Vt,x),vn(n,i,u,r),u=et?Yr:0,!M&&n!==null&&(n.flags&128)!==0)e:for(n=i.child;n!==null;){if(n.tag===13)n.memoizedState!==null&&j_(n,r,i);else if(n.tag===19)j_(n,r,i);else if(n.child!==null){n.child.return=n,n=n.child;continue}if(n===i)break e;for(;n.sibling===null;){if(n.return===null||n.return===i)break e;n=n.return}n.sibling.return=n.return,n=n.sibling}switch(h){case"forwards":for(r=i.child,h=null;r!==null;)n=r.alternate,n!==null&&Rc(n)===null&&(h=r),r=r.sibling;r=h,r===null?(h=i.child,i.child=null):(h=r.sibling,r.sibling=null),yp(i,!1,h,r,g,u);break;case"backwards":case"unstable_legacy-backwards":for(r=null,h=i.child,i.child=null;h!==null;){if(n=h.alternate,n!==null&&Rc(n)===null){i.child=h;break}n=h.sibling,h.sibling=r,r=h,h=n}yp(i,!0,r,null,g,u);break;case"together":yp(i,!1,null,null,void 0,u);break;default:i.memoizedState=null}return i.child}function pi(n,i,r){if(n!==null&&(i.dependencies=n.dependencies),Qi|=i.lanes,(r&i.childLanes)===0)if(n!==null){if(Mo(n,i,r,!1),(r&i.childLanes)===0)return null}else return null;if(n!==null&&i.child!==n.child)throw Error(a(153));if(i.child!==null){for(n=i.child,r=ri(n,n.pendingProps),i.child=r,r.return=i;n.sibling!==null;)n=n.sibling,r=r.sibling=ri(n,n.pendingProps),r.return=i;r.sibling=null}return i.child}function _p(n,i){return(n.lanes&i)!==0?!0:(n=n.dependencies,!!(n!==null&&Tc(n)))}function Cj(n,i,r){switch(i.tag){case 3:P(i,i.stateNode.containerInfo),Pi(i,Wt,n.memoizedState.cache),La();break;case 27:case 5:Se(i);break;case 4:P(i,i.stateNode.containerInfo);break;case 10:Pi(i,i.type,i.memoizedProps.value);break;case 31:if(i.memoizedState!==null)return i.flags|=128,Zf(i),null;break;case 13:var u=i.memoizedState;if(u!==null)return u.dehydrated!==null?(Gi(i),i.flags|=128,null):(r&i.child.childLanes)!==0?k_(n,i,r):(Gi(i),n=pi(n,i,r),n!==null?n.sibling:null);Gi(i);break;case 19:var h=(n.flags&128)!==0;if(u=(r&i.childLanes)!==0,u||(Mo(n,i,r,!1),u=(r&i.childLanes)!==0),h){if(u)return N_(n,i,r);i.flags|=128}if(h=i.memoizedState,h!==null&&(h.rendering=null,h.tail=null,h.lastEffect=null),ce(Vt,Vt.current),u)break;return null;case 22:return i.lanes=0,__(n,i,r,i.pendingProps);case 24:Pi(i,Wt,n.memoizedState.cache)}return pi(n,i,r)}function E_(n,i,r){if(n!==null)if(n.memoizedProps!==i.pendingProps)tn=!0;else{if(!_p(n,r)&&(i.flags&128)===0)return tn=!1,Cj(n,i,r);tn=(n.flags&131072)!==0}else tn=!1,et&&(i.flags&1048576)!==0&&ay(i,Yr,i.index);switch(i.lanes=0,i.tag){case 16:e:{var u=i.pendingProps;if(n=Ha(i.elementType),i.type=n,typeof n=="function")Sf(n)?(u=Ya(n,u),i.tag=1,i=w_(null,i,n,u,r)):(i.tag=0,i=dp(null,i,n,u,r));else{if(n!=null){var h=n.$$typeof;if(h===T){i.tag=11,i=h_(null,i,n,u,r);break e}else if(h===R){i.tag=14,i=g_(null,i,n,u,r);break e}}throw i=L(n)||n,Error(a(306,i,""))}}return i;case 0:return dp(n,i,i.type,i.pendingProps,r);case 1:return u=i.type,h=Ya(u,i.pendingProps),w_(n,i,u,h,r);case 3:e:{if(P(i,i.stateNode.containerInfo),n===null)throw Error(a(387));u=i.pendingProps;var g=i.memoizedState;h=g.element,qf(n,i),Wr(i,u,null,r);var x=i.memoizedState;if(u=x.cache,Pi(i,Wt,u),u!==g.cache&&Mf(i,[Wt],r,!0),Qr(),u=x.element,g.isDehydrated)if(g={element:u,isDehydrated:!1,cache:x.cache},i.updateQueue.baseState=g,i.memoizedState=g,i.flags&256){i=S_(n,i,u,r);break e}else if(u!==h){h=cs(Error(a(424)),i),Vr(h),i=S_(n,i,u,r);break e}else for(n=i.stateNode.containerInfo,n.nodeType===9?n=n.body:n=n.nodeName==="HTML"?n.ownerDocument.body:n,jt=ms(n.firstChild),_n=i,et=!0,Ii=null,fs=!0,r=_y(i,null,u,r),i.child=r;r;)r.flags=r.flags&-3|4096,r=r.sibling;else{if(La(),u===h){i=pi(n,i,r);break e}vn(n,i,u,r)}i=i.child}return i;case 26:return Kc(n,i),n===null?(r=qb(i.type,null,i.pendingProps,null))?i.memoizedState=r:et||(r=i.type,n=i.pendingProps,u=uu(F.current).createElement(r),u[kt]=i,u[Yt]=n,xn(u,r,n),Dt(u),i.stateNode=u):i.memoizedState=qb(i.type,n.memoizedProps,i.pendingProps,n.memoizedState),null;case 27:return Se(i),n===null&&et&&(u=i.stateNode=$b(i.type,i.pendingProps,F.current),_n=i,fs=!0,h=jt,sa(i.type)?(Jp=h,jt=ms(u.firstChild)):jt=h),vn(n,i,i.pendingProps.children,r),Kc(n,i),n===null&&(i.flags|=4194304),i.child;case 5:return n===null&&et&&((h=u=jt)&&(u=aN(u,i.type,i.pendingProps,fs),u!==null?(i.stateNode=u,_n=i,jt=ms(u.firstChild),fs=!1,h=!0):h=!1),h||Zi(i)),Se(i),h=i.type,g=i.pendingProps,x=n!==null?n.memoizedProps:null,u=g.children,Vp(h,g)?u=null:x!==null&&Vp(h,x)&&(i.flags|=32),i.memoizedState!==null&&(h=Yf(n,i,vj,null,null,r),bl._currentValue=h),Kc(n,i),vn(n,i,u,r),i.child;case 6:return n===null&&et&&((n=r=jt)&&(r=oN(r,i.pendingProps,fs),r!==null?(i.stateNode=r,_n=i,jt=null,n=!0):n=!1),n||Zi(i)),null;case 13:return k_(n,i,r);case 4:return P(i,i.stateNode.containerInfo),u=i.pendingProps,n===null?i.child=Za(i,null,u,r):vn(n,i,u,r),i.child;case 11:return h_(n,i,i.type,i.pendingProps,r);case 7:return vn(n,i,i.pendingProps,r),i.child;case 8:return vn(n,i,i.pendingProps.children,r),i.child;case 12:return vn(n,i,i.pendingProps.children,r),i.child;case 10:return u=i.pendingProps,Pi(i,i.type,u.value),vn(n,i,u.children,r),i.child;case 9:return h=i.type._context,u=i.pendingProps.children,Ua(i),h=bn(h),u=u(h),i.flags|=1,vn(n,i,u,r),i.child;case 14:return g_(n,i,i.type,i.pendingProps,r);case 15:return y_(n,i,i.type,i.pendingProps,r);case 19:return N_(n,i,r);case 31:return Tj(n,i,r);case 22:return __(n,i,r,i.pendingProps);case 24:return Ua(i),u=bn(Wt),n===null?(h=$f(),h===null&&(h=_t,g=zf(),h.pooledCache=g,g.refCount++,g!==null&&(h.pooledCacheLanes|=r),h=g),i.memoizedState={parent:u,cache:h},Lf(i),Pi(i,Wt,h)):((n.lanes&r)!==0&&(qf(n,i),Wr(i,null,null,r),Qr()),h=n.memoizedState,g=i.memoizedState,h.parent!==u?(h={parent:u,cache:u},i.memoizedState=h,i.lanes===0&&(i.memoizedState=i.updateQueue.baseState=h),Pi(i,Wt,u)):(u=g.cache,Pi(i,Wt,u),u!==h.cache&&Mf(i,[Wt],r,!0))),vn(n,i,i.pendingProps.children,r),i.child;case 29:throw i.pendingProps}throw Error(a(156,i.tag))}function mi(n){n.flags|=4}function bp(n,i,r,u,h){if((i=(n.mode&32)!==0)&&(i=!1),i){if(n.flags|=16777216,(h&335544128)===h)if(n.stateNode.complete)n.flags|=8192;else if(eb())n.flags|=8192;else throw Ia=Mc,Rf}else n.flags&=-16777217}function T_(n,i){if(i.type!=="stylesheet"||(i.state.loading&4)!==0)n.flags&=-16777217;else if(n.flags|=16777216,!Zb(i))if(eb())n.flags|=8192;else throw Ia=Mc,Rf}function Xc(n,i){i!==null&&(n.flags|=4),n.flags&16384&&(i=n.tag!==22?Ws():536870912,n.lanes|=i,Po|=i)}function al(n,i){if(!et)switch(n.tailMode){case"hidden":i=n.tail;for(var r=null;i!==null;)i.alternate!==null&&(r=i),i=i.sibling;r===null?n.tail=null:r.sibling=null;break;case"collapsed":r=n.tail;for(var u=null;r!==null;)r.alternate!==null&&(u=r),r=r.sibling;u===null?i||n.tail===null?n.tail=null:n.tail.sibling=null:u.sibling=null}}function Nt(n){var i=n.alternate!==null&&n.alternate.child===n.child,r=0,u=0;if(i)for(var h=n.child;h!==null;)r|=h.lanes|h.childLanes,u|=h.subtreeFlags&65011712,u|=h.flags&65011712,h.return=n,h=h.sibling;else for(h=n.child;h!==null;)r|=h.lanes|h.childLanes,u|=h.subtreeFlags,u|=h.flags,h.return=n,h=h.sibling;return n.subtreeFlags|=u,n.childLanes=r,i}function Aj(n,i,r){var u=i.pendingProps;switch(Ef(i),i.tag){case 16:case 15:case 0:case 11:case 7:case 8:case 12:case 9:case 14:return Nt(i),null;case 1:return Nt(i),null;case 3:return r=i.stateNode,u=null,n!==null&&(u=n.memoizedState.cache),i.memoizedState.cache!==u&&(i.flags|=2048),ui(Wt),ae(),r.pendingContext&&(r.context=r.pendingContext,r.pendingContext=null),(n===null||n.child===null)&&(Oo(i)?mi(i):n===null||n.memoizedState.isDehydrated&&(i.flags&256)===0||(i.flags|=1024,Cf())),Nt(i),null;case 26:var h=i.type,g=i.memoizedState;return n===null?(mi(i),g!==null?(Nt(i),T_(i,g)):(Nt(i),bp(i,h,null,u,r))):g?g!==n.memoizedState?(mi(i),Nt(i),T_(i,g)):(Nt(i),i.flags&=-16777217):(n=n.memoizedProps,n!==u&&mi(i),Nt(i),bp(i,h,n,u,r)),null;case 27:if(je(i),r=F.current,h=i.type,n!==null&&i.stateNode!=null)n.memoizedProps!==u&&mi(i);else{if(!u){if(i.stateNode===null)throw Error(a(166));return Nt(i),null}n=de.current,Oo(i)?ry(i):(n=$b(h,u,r),i.stateNode=n,mi(i))}return Nt(i),null;case 5:if(je(i),h=i.type,n!==null&&i.stateNode!=null)n.memoizedProps!==u&&mi(i);else{if(!u){if(i.stateNode===null)throw Error(a(166));return Nt(i),null}if(g=de.current,Oo(i))ry(i);else{var x=uu(F.current);switch(g){case 1:g=x.createElementNS("http://www.w3.org/2000/svg",h);break;case 2:g=x.createElementNS("http://www.w3.org/1998/Math/MathML",h);break;default:switch(h){case"svg":g=x.createElementNS("http://www.w3.org/2000/svg",h);break;case"math":g=x.createElementNS("http://www.w3.org/1998/Math/MathML",h);break;case"script":g=x.createElement("div"),g.innerHTML="<script><\/script>",g=g.removeChild(g.firstChild);break;case"select":g=typeof u.is=="string"?x.createElement("select",{is:u.is}):x.createElement("select"),u.multiple?g.multiple=!0:u.size&&(g.size=u.size);break;default:g=typeof u.is=="string"?x.createElement(h,{is:u.is}):x.createElement(h)}}g[kt]=i,g[Yt]=u;e:for(x=i.child;x!==null;){if(x.tag===5||x.tag===6)g.appendChild(x.stateNode);else if(x.tag!==4&&x.tag!==27&&x.child!==null){x.child.return=x,x=x.child;continue}if(x===i)break e;for(;x.sibling===null;){if(x.return===null||x.return===i)break e;x=x.return}x.sibling.return=x.return,x=x.sibling}i.stateNode=g;e:switch(xn(g,h,u),h){case"button":case"input":case"select":case"textarea":u=!!u.autoFocus;break e;case"img":u=!0;break e;default:u=!1}u&&mi(i)}}return Nt(i),bp(i,i.type,n===null?null:n.memoizedProps,i.pendingProps,r),null;case 6:if(n&&i.stateNode!=null)n.memoizedProps!==u&&mi(i);else{if(typeof u!="string"&&i.stateNode===null)throw Error(a(166));if(n=F.current,Oo(i)){if(n=i.stateNode,r=i.memoizedProps,u=null,h=_n,h!==null)switch(h.tag){case 27:case 5:u=h.memoizedProps}n[kt]=i,n=!!(n.nodeValue===r||u!==null&&u.suppressHydrationWarning===!0||kb(n.nodeValue,r)),n||Zi(i,!0)}else n=uu(n).createTextNode(u),n[kt]=i,i.stateNode=n}return Nt(i),null;case 31:if(r=i.memoizedState,n===null||n.memoizedState!==null){if(u=Oo(i),r!==null){if(n===null){if(!u)throw Error(a(318));if(n=i.memoizedState,n=n!==null?n.dehydrated:null,!n)throw Error(a(557));n[kt]=i}else La(),(i.flags&128)===0&&(i.memoizedState=null),i.flags|=4;Nt(i),n=!1}else r=Cf(),n!==null&&n.memoizedState!==null&&(n.memoizedState.hydrationErrors=r),n=!0;if(!n)return i.flags&256?(Jn(i),i):(Jn(i),null);if((i.flags&128)!==0)throw Error(a(558))}return Nt(i),null;case 13:if(u=i.memoizedState,n===null||n.memoizedState!==null&&n.memoizedState.dehydrated!==null){if(h=Oo(i),u!==null&&u.dehydrated!==null){if(n===null){if(!h)throw Error(a(318));if(h=i.memoizedState,h=h!==null?h.dehydrated:null,!h)throw Error(a(317));h[kt]=i}else La(),(i.flags&128)===0&&(i.memoizedState=null),i.flags|=4;Nt(i),h=!1}else h=Cf(),n!==null&&n.memoizedState!==null&&(n.memoizedState.hydrationErrors=h),h=!0;if(!h)return i.flags&256?(Jn(i),i):(Jn(i),null)}return Jn(i),(i.flags&128)!==0?(i.lanes=r,i):(r=u!==null,n=n!==null&&n.memoizedState!==null,r&&(u=i.child,h=null,u.alternate!==null&&u.alternate.memoizedState!==null&&u.alternate.memoizedState.cachePool!==null&&(h=u.alternate.memoizedState.cachePool.pool),g=null,u.memoizedState!==null&&u.memoizedState.cachePool!==null&&(g=u.memoizedState.cachePool.pool),g!==h&&(u.flags|=2048)),r!==n&&r&&(i.child.flags|=8192),Xc(i,i.updateQueue),Nt(i),null);case 4:return ae(),n===null&&Hp(i.stateNode.containerInfo),Nt(i),null;case 10:return ui(i.type),Nt(i),null;case 19:if(Y(Vt),u=i.memoizedState,u===null)return Nt(i),null;if(h=(i.flags&128)!==0,g=u.rendering,g===null)if(h)al(u,!1);else{if(Zt!==0||n!==null&&(n.flags&128)!==0)for(n=i.child;n!==null;){if(g=Rc(n),g!==null){for(i.flags|=128,al(u,!1),n=g.updateQueue,i.updateQueue=n,Xc(i,n),i.subtreeFlags=0,n=r,r=i.child;r!==null;)ny(r,n),r=r.sibling;return ce(Vt,Vt.current&1|2),et&&li(i,u.treeForkCount),i.child}n=n.sibling}u.tail!==null&&lt()>eu&&(i.flags|=128,h=!0,al(u,!1),i.lanes=4194304)}else{if(!h)if(n=Rc(g),n!==null){if(i.flags|=128,h=!0,n=n.updateQueue,i.updateQueue=n,Xc(i,n),al(u,!0),u.tail===null&&u.tailMode==="hidden"&&!g.alternate&&!et)return Nt(i),null}else 2*lt()-u.renderingStartTime>eu&&r!==536870912&&(i.flags|=128,h=!0,al(u,!1),i.lanes=4194304);u.isBackwards?(g.sibling=i.child,i.child=g):(n=u.last,n!==null?n.sibling=g:i.child=g,u.last=g)}return u.tail!==null?(n=u.tail,u.rendering=n,u.tail=n.sibling,u.renderingStartTime=lt(),n.sibling=null,r=Vt.current,ce(Vt,h?r&1|2:r&1),et&&li(i,u.treeForkCount),n):(Nt(i),null);case 22:case 23:return Jn(i),If(),u=i.memoizedState!==null,n!==null?n.memoizedState!==null!==u&&(i.flags|=8192):u&&(i.flags|=8192),u?(r&536870912)!==0&&(i.flags&128)===0&&(Nt(i),i.subtreeFlags&6&&(i.flags|=8192)):Nt(i),r=i.updateQueue,r!==null&&Xc(i,r.retryQueue),r=null,n!==null&&n.memoizedState!==null&&n.memoizedState.cachePool!==null&&(r=n.memoizedState.cachePool.pool),u=null,i.memoizedState!==null&&i.memoizedState.cachePool!==null&&(u=i.memoizedState.cachePool.pool),u!==r&&(i.flags|=2048),n!==null&&Y(Ba),null;case 24:return r=null,n!==null&&(r=n.memoizedState.cache),i.memoizedState.cache!==r&&(i.flags|=2048),ui(Wt),Nt(i),null;case 25:return null;case 30:return null}throw Error(a(156,i.tag))}function Oj(n,i){switch(Ef(i),i.tag){case 1:return n=i.flags,n&65536?(i.flags=n&-65537|128,i):null;case 3:return ui(Wt),ae(),n=i.flags,(n&65536)!==0&&(n&128)===0?(i.flags=n&-65537|128,i):null;case 26:case 27:case 5:return je(i),null;case 31:if(i.memoizedState!==null){if(Jn(i),i.alternate===null)throw Error(a(340));La()}return n=i.flags,n&65536?(i.flags=n&-65537|128,i):null;case 13:if(Jn(i),n=i.memoizedState,n!==null&&n.dehydrated!==null){if(i.alternate===null)throw Error(a(340));La()}return n=i.flags,n&65536?(i.flags=n&-65537|128,i):null;case 19:return Y(Vt),null;case 4:return ae(),null;case 10:return ui(i.type),null;case 22:case 23:return Jn(i),If(),n!==null&&Y(Ba),n=i.flags,n&65536?(i.flags=n&-65537|128,i):null;case 24:return ui(Wt),null;case 25:return null;default:return null}}function C_(n,i){switch(Ef(i),i.tag){case 3:ui(Wt),ae();break;case 26:case 27:case 5:je(i);break;case 4:ae();break;case 31:i.memoizedState!==null&&Jn(i);break;case 13:Jn(i);break;case 19:Y(Vt);break;case 10:ui(i.type);break;case 22:case 23:Jn(i),If(),n!==null&&Y(Ba);break;case 24:ui(Wt)}}function ol(n,i){try{var r=i.updateQueue,u=r!==null?r.lastEffect:null;if(u!==null){var h=u.next;r=h;do{if((r.tag&n)===n){u=void 0;var g=r.create,x=r.inst;u=g(),x.destroy=u}r=r.next}while(r!==h)}}catch(M){pt(i,i.return,M)}}function Fi(n,i,r){try{var u=i.updateQueue,h=u!==null?u.lastEffect:null;if(h!==null){var g=h.next;u=g;do{if((u.tag&n)===n){var x=u.inst,M=x.destroy;if(M!==void 0){x.destroy=void 0,h=i;var H=r,se=M;try{se()}catch(fe){pt(h,H,fe)}}}u=u.next}while(u!==g)}}catch(fe){pt(i,i.return,fe)}}function A_(n){var i=n.updateQueue;if(i!==null){var r=n.stateNode;try{vy(i,r)}catch(u){pt(n,n.return,u)}}}function O_(n,i,r){r.props=Ya(n.type,n.memoizedProps),r.state=n.memoizedState;try{r.componentWillUnmount()}catch(u){pt(n,i,u)}}function rl(n,i){try{var r=n.ref;if(r!==null){switch(n.tag){case 26:case 27:case 5:var u=n.stateNode;break;case 30:u=n.stateNode;break;default:u=n.stateNode}typeof r=="function"?n.refCleanup=r(u):r.current=u}}catch(h){pt(n,i,h)}}function Ys(n,i){var r=n.ref,u=n.refCleanup;if(r!==null)if(typeof u=="function")try{u()}catch(h){pt(n,i,h)}finally{n.refCleanup=null,n=n.alternate,n!=null&&(n.refCleanup=null)}else if(typeof r=="function")try{r(null)}catch(h){pt(n,i,h)}else r.current=null}function M_(n){var i=n.type,r=n.memoizedProps,u=n.stateNode;try{e:switch(i){case"button":case"input":case"select":case"textarea":r.autoFocus&&u.focus();break e;case"img":r.src?u.src=r.src:r.srcSet&&(u.srcset=r.srcSet)}}catch(h){pt(n,n.return,h)}}function vp(n,i,r){try{var u=n.stateNode;Wj(u,n.type,r,i),u[Yt]=i}catch(h){pt(n,n.return,h)}}function z_(n){return n.tag===5||n.tag===3||n.tag===26||n.tag===27&&sa(n.type)||n.tag===4}function xp(n){e:for(;;){for(;n.sibling===null;){if(n.return===null||z_(n.return))return null;n=n.return}for(n.sibling.return=n.return,n=n.sibling;n.tag!==5&&n.tag!==6&&n.tag!==18;){if(n.tag===27&&sa(n.type)||n.flags&2||n.child===null||n.tag===4)continue e;n.child.return=n,n=n.child}if(!(n.flags&2))return n.stateNode}}function wp(n,i,r){var u=n.tag;if(u===5||u===6)n=n.stateNode,i?(r.nodeType===9?r.body:r.nodeName==="HTML"?r.ownerDocument.body:r).insertBefore(n,i):(i=r.nodeType===9?r.body:r.nodeName==="HTML"?r.ownerDocument.body:r,i.appendChild(n),r=r._reactRootContainer,r!=null||i.onclick!==null||(i.onclick=be));else if(u!==4&&(u===27&&sa(n.type)&&(r=n.stateNode,i=null),n=n.child,n!==null))for(wp(n,i,r),n=n.sibling;n!==null;)wp(n,i,r),n=n.sibling}function Fc(n,i,r){var u=n.tag;if(u===5||u===6)n=n.stateNode,i?r.insertBefore(n,i):r.appendChild(n);else if(u!==4&&(u===27&&sa(n.type)&&(r=n.stateNode),n=n.child,n!==null))for(Fc(n,i,r),n=n.sibling;n!==null;)Fc(n,i,r),n=n.sibling}function D_(n){var i=n.stateNode,r=n.memoizedProps;try{for(var u=n.type,h=i.attributes;h.length;)i.removeAttributeNode(h[0]);xn(i,u,r),i[kt]=n,i[Yt]=r}catch(g){pt(n,n.return,g)}}var hi=!1,nn=!1,Sp=!1,$_=typeof WeakSet=="function"?WeakSet:Set,fn=null;function Mj(n,i){if(n=n.containerInfo,Pp=yu,n=Kg(n),gf(n)){if("selectionStart"in n)var r={start:n.selectionStart,end:n.selectionEnd};else e:{r=(r=n.ownerDocument)&&r.defaultView||window;var u=r.getSelection&&r.getSelection();if(u&&u.rangeCount!==0){r=u.anchorNode;var h=u.anchorOffset,g=u.focusNode;u=u.focusOffset;try{r.nodeType,g.nodeType}catch{r=null;break e}var x=0,M=-1,H=-1,se=0,fe=0,ye=n,ie=null;t:for(;;){for(var re;ye!==r||h!==0&&ye.nodeType!==3||(M=x+h),ye!==g||u!==0&&ye.nodeType!==3||(H=x+u),ye.nodeType===3&&(x+=ye.nodeValue.length),(re=ye.firstChild)!==null;)ie=ye,ye=re;for(;;){if(ye===n)break t;if(ie===r&&++se===h&&(M=x),ie===g&&++fe===u&&(H=x),(re=ye.nextSibling)!==null)break;ye=ie,ie=ye.parentNode}ye=re}r=M===-1||H===-1?null:{start:M,end:H}}else r=null}r=r||{start:0,end:0}}else r=null;for(Yp={focusedElem:n,selectionRange:r},yu=!1,fn=i;fn!==null;)if(i=fn,n=i.child,(i.subtreeFlags&1028)!==0&&n!==null)n.return=i,fn=n;else for(;fn!==null;){switch(i=fn,g=i.alternate,n=i.flags,i.tag){case 0:if((n&4)!==0&&(n=i.updateQueue,n=n!==null?n.events:null,n!==null))for(r=0;r<n.length;r++)h=n[r],h.ref.impl=h.nextImpl;break;case 11:case 15:break;case 1:if((n&1024)!==0&&g!==null){n=void 0,r=i,h=g.memoizedProps,g=g.memoizedState,u=r.stateNode;try{var Ee=Ya(r.type,h);n=u.getSnapshotBeforeUpdate(Ee,g),u.__reactInternalSnapshotBeforeUpdate=n}catch(Le){pt(r,r.return,Le)}}break;case 3:if((n&1024)!==0){if(n=i.stateNode.containerInfo,r=n.nodeType,r===9)Gp(n);else if(r===1)switch(n.nodeName){case"HEAD":case"HTML":case"BODY":Gp(n);break;default:n.textContent=""}}break;case 5:case 26:case 27:case 6:case 4:case 17:break;default:if((n&1024)!==0)throw Error(a(163))}if(n=i.sibling,n!==null){n.return=i.return,fn=n;break}fn=i.return}}function R_(n,i,r){var u=r.flags;switch(r.tag){case 0:case 11:case 15:yi(n,r),u&4&&ol(5,r);break;case 1:if(yi(n,r),u&4)if(n=r.stateNode,i===null)try{n.componentDidMount()}catch(x){pt(r,r.return,x)}else{var h=Ya(r.type,i.memoizedProps);i=i.memoizedState;try{n.componentDidUpdate(h,i,n.__reactInternalSnapshotBeforeUpdate)}catch(x){pt(r,r.return,x)}}u&64&&A_(r),u&512&&rl(r,r.return);break;case 3:if(yi(n,r),u&64&&(n=r.updateQueue,n!==null)){if(i=null,r.child!==null)switch(r.child.tag){case 27:case 5:i=r.child.stateNode;break;case 1:i=r.child.stateNode}try{vy(n,i)}catch(x){pt(r,r.return,x)}}break;case 27:i===null&&u&4&&D_(r);case 26:case 5:yi(n,r),i===null&&u&4&&M_(r),u&512&&rl(r,r.return);break;case 12:yi(n,r);break;case 31:yi(n,r),u&4&&U_(n,r);break;case 13:yi(n,r),u&4&&B_(n,r),u&64&&(n=r.memoizedState,n!==null&&(n=n.dehydrated,n!==null&&(r=Hj.bind(null,r),rN(n,r))));break;case 22:if(u=r.memoizedState!==null||hi,!u){i=i!==null&&i.memoizedState!==null||nn,h=hi;var g=nn;hi=u,(nn=i)&&!g?_i(n,r,(r.subtreeFlags&8772)!==0):yi(n,r),hi=h,nn=g}break;case 30:break;default:yi(n,r)}}function L_(n){var i=n.alternate;i!==null&&(n.alternate=null,L_(i)),n.child=null,n.deletions=null,n.sibling=null,n.tag===5&&(i=n.stateNode,i!==null&&yn(i)),n.stateNode=null,n.return=null,n.dependencies=null,n.memoizedProps=null,n.memoizedState=null,n.pendingProps=null,n.stateNode=null,n.updateQueue=null}var Ct=null,Ln=!1;function gi(n,i,r){for(r=r.child;r!==null;)q_(n,i,r),r=r.sibling}function q_(n,i,r){if(Qt&&typeof Qt.onCommitFiberUnmount=="function")try{Qt.onCommitFiberUnmount(Dn,r)}catch{}switch(r.tag){case 26:nn||Ys(r,i),gi(n,i,r),r.memoizedState?r.memoizedState.count--:r.stateNode&&(r=r.stateNode,r.parentNode.removeChild(r));break;case 27:nn||Ys(r,i);var u=Ct,h=Ln;sa(r.type)&&(Ct=r.stateNode,Ln=!1),gi(n,i,r),gl(r.stateNode),Ct=u,Ln=h;break;case 5:nn||Ys(r,i);case 6:if(u=Ct,h=Ln,Ct=null,gi(n,i,r),Ct=u,Ln=h,Ct!==null)if(Ln)try{(Ct.nodeType===9?Ct.body:Ct.nodeName==="HTML"?Ct.ownerDocument.body:Ct).removeChild(r.stateNode)}catch(g){pt(r,i,g)}else try{Ct.removeChild(r.stateNode)}catch(g){pt(r,i,g)}break;case 18:Ct!==null&&(Ln?(n=Ct,Ab(n.nodeType===9?n.body:n.nodeName==="HTML"?n.ownerDocument.body:n,r.stateNode),Qo(n)):Ab(Ct,r.stateNode));break;case 4:u=Ct,h=Ln,Ct=r.stateNode.containerInfo,Ln=!0,gi(n,i,r),Ct=u,Ln=h;break;case 0:case 11:case 14:case 15:Fi(2,r,i),nn||Fi(4,r,i),gi(n,i,r);break;case 1:nn||(Ys(r,i),u=r.stateNode,typeof u.componentWillUnmount=="function"&&O_(r,i,u)),gi(n,i,r);break;case 21:gi(n,i,r);break;case 22:nn=(u=nn)||r.memoizedState!==null,gi(n,i,r),nn=u;break;default:gi(n,i,r)}}function U_(n,i){if(i.memoizedState===null&&(n=i.alternate,n!==null&&(n=n.memoizedState,n!==null))){n=n.dehydrated;try{Qo(n)}catch(r){pt(i,i.return,r)}}}function B_(n,i){if(i.memoizedState===null&&(n=i.alternate,n!==null&&(n=n.memoizedState,n!==null&&(n=n.dehydrated,n!==null))))try{Qo(n)}catch(r){pt(i,i.return,r)}}function zj(n){switch(n.tag){case 31:case 13:case 19:var i=n.stateNode;return i===null&&(i=n.stateNode=new $_),i;case 22:return n=n.stateNode,i=n._retryCache,i===null&&(i=n._retryCache=new $_),i;default:throw Error(a(435,n.tag))}}function Jc(n,i){var r=zj(n);i.forEach(function(u){if(!r.has(u)){r.add(u);var h=Ij.bind(null,n,u);u.then(h,h)}})}function qn(n,i){var r=i.deletions;if(r!==null)for(var u=0;u<r.length;u++){var h=r[u],g=n,x=i,M=x;e:for(;M!==null;){switch(M.tag){case 27:if(sa(M.type)){Ct=M.stateNode,Ln=!1;break e}break;case 5:Ct=M.stateNode,Ln=!1;break e;case 3:case 4:Ct=M.stateNode.containerInfo,Ln=!0;break e}M=M.return}if(Ct===null)throw Error(a(160));q_(g,x,h),Ct=null,Ln=!1,g=h.alternate,g!==null&&(g.return=null),h.return=null}if(i.subtreeFlags&13886)for(i=i.child;i!==null;)H_(i,n),i=i.sibling}var Ns=null;function H_(n,i){var r=n.alternate,u=n.flags;switch(n.tag){case 0:case 11:case 14:case 15:qn(i,n),Un(n),u&4&&(Fi(3,n,n.return),ol(3,n),Fi(5,n,n.return));break;case 1:qn(i,n),Un(n),u&512&&(nn||r===null||Ys(r,r.return)),u&64&&hi&&(n=n.updateQueue,n!==null&&(u=n.callbacks,u!==null&&(r=n.shared.hiddenCallbacks,n.shared.hiddenCallbacks=r===null?u:r.concat(u))));break;case 26:var h=Ns;if(qn(i,n),Un(n),u&512&&(nn||r===null||Ys(r,r.return)),u&4){var g=r!==null?r.memoizedState:null;if(u=n.memoizedState,r===null)if(u===null)if(n.stateNode===null){e:{u=n.type,r=n.memoizedProps,h=h.ownerDocument||h;t:switch(u){case"title":g=h.getElementsByTagName("title")[0],(!g||g[gn]||g[kt]||g.namespaceURI==="http://www.w3.org/2000/svg"||g.hasAttribute("itemprop"))&&(g=h.createElement(u),h.head.insertBefore(g,h.querySelector("head > title"))),xn(g,u,r),g[kt]=n,Dt(g),u=g;break e;case"link":var x=Hb("link","href",h).get(u+(r.href||""));if(x){for(var M=0;M<x.length;M++)if(g=x[M],g.getAttribute("href")===(r.href==null||r.href===""?null:r.href)&&g.getAttribute("rel")===(r.rel==null?null:r.rel)&&g.getAttribute("title")===(r.title==null?null:r.title)&&g.getAttribute("crossorigin")===(r.crossOrigin==null?null:r.crossOrigin)){x.splice(M,1);break t}}g=h.createElement(u),xn(g,u,r),h.head.appendChild(g);break;case"meta":if(x=Hb("meta","content",h).get(u+(r.content||""))){for(M=0;M<x.length;M++)if(g=x[M],g.getAttribute("content")===(r.content==null?null:""+r.content)&&g.getAttribute("name")===(r.name==null?null:r.name)&&g.getAttribute("property")===(r.property==null?null:r.property)&&g.getAttribute("http-equiv")===(r.httpEquiv==null?null:r.httpEquiv)&&g.getAttribute("charset")===(r.charSet==null?null:r.charSet)){x.splice(M,1);break t}}g=h.createElement(u),xn(g,u,r),h.head.appendChild(g);break;default:throw Error(a(468,u))}g[kt]=n,Dt(g),u=g}n.stateNode=u}else Ib(h,n.type,n.stateNode);else n.stateNode=Bb(h,u,n.memoizedProps);else g!==u?(g===null?r.stateNode!==null&&(r=r.stateNode,r.parentNode.removeChild(r)):g.count--,u===null?Ib(h,n.type,n.stateNode):Bb(h,u,n.memoizedProps)):u===null&&n.stateNode!==null&&vp(n,n.memoizedProps,r.memoizedProps)}break;case 27:qn(i,n),Un(n),u&512&&(nn||r===null||Ys(r,r.return)),r!==null&&u&4&&vp(n,n.memoizedProps,r.memoizedProps);break;case 5:if(qn(i,n),Un(n),u&512&&(nn||r===null||Ys(r,r.return)),n.flags&32){h=n.stateNode;try{ai(h,"")}catch(Ee){pt(n,n.return,Ee)}}u&4&&n.stateNode!=null&&(h=n.memoizedProps,vp(n,h,r!==null?r.memoizedProps:h)),u&1024&&(Sp=!0);break;case 6:if(qn(i,n),Un(n),u&4){if(n.stateNode===null)throw Error(a(162));u=n.memoizedProps,r=n.stateNode;try{r.nodeValue=u}catch(Ee){pt(n,n.return,Ee)}}break;case 3:if(pu=null,h=Ns,Ns=du(i.containerInfo),qn(i,n),Ns=h,Un(n),u&4&&r!==null&&r.memoizedState.isDehydrated)try{Qo(i.containerInfo)}catch(Ee){pt(n,n.return,Ee)}Sp&&(Sp=!1,I_(n));break;case 4:u=Ns,Ns=du(n.stateNode.containerInfo),qn(i,n),Un(n),Ns=u;break;case 12:qn(i,n),Un(n);break;case 31:qn(i,n),Un(n),u&4&&(u=n.updateQueue,u!==null&&(n.updateQueue=null,Jc(n,u)));break;case 13:qn(i,n),Un(n),n.child.flags&8192&&n.memoizedState!==null!=(r!==null&&r.memoizedState!==null)&&(Wc=lt()),u&4&&(u=n.updateQueue,u!==null&&(n.updateQueue=null,Jc(n,u)));break;case 22:h=n.memoizedState!==null;var H=r!==null&&r.memoizedState!==null,se=hi,fe=nn;if(hi=se||h,nn=fe||H,qn(i,n),nn=fe,hi=se,Un(n),u&8192)e:for(i=n.stateNode,i._visibility=h?i._visibility&-2:i._visibility|1,h&&(r===null||H||hi||nn||Va(n)),r=null,i=n;;){if(i.tag===5||i.tag===26){if(r===null){H=r=i;try{if(g=H.stateNode,h)x=g.style,typeof x.setProperty=="function"?x.setProperty("display","none","important"):x.display="none";else{M=H.stateNode;var ye=H.memoizedProps.style,ie=ye!=null&&ye.hasOwnProperty("display")?ye.display:null;M.style.display=ie==null||typeof ie=="boolean"?"":(""+ie).trim()}}catch(Ee){pt(H,H.return,Ee)}}}else if(i.tag===6){if(r===null){H=i;try{H.stateNode.nodeValue=h?"":H.memoizedProps}catch(Ee){pt(H,H.return,Ee)}}}else if(i.tag===18){if(r===null){H=i;try{var re=H.stateNode;h?Ob(re,!0):Ob(H.stateNode,!1)}catch(Ee){pt(H,H.return,Ee)}}}else if((i.tag!==22&&i.tag!==23||i.memoizedState===null||i===n)&&i.child!==null){i.child.return=i,i=i.child;continue}if(i===n)break e;for(;i.sibling===null;){if(i.return===null||i.return===n)break e;r===i&&(r=null),i=i.return}r===i&&(r=null),i.sibling.return=i.return,i=i.sibling}u&4&&(u=n.updateQueue,u!==null&&(r=u.retryQueue,r!==null&&(u.retryQueue=null,Jc(n,r))));break;case 19:qn(i,n),Un(n),u&4&&(u=n.updateQueue,u!==null&&(n.updateQueue=null,Jc(n,u)));break;case 30:break;case 21:break;default:qn(i,n),Un(n)}}function Un(n){var i=n.flags;if(i&2){try{for(var r,u=n.return;u!==null;){if(z_(u)){r=u;break}u=u.return}if(r==null)throw Error(a(160));switch(r.tag){case 27:var h=r.stateNode,g=xp(n);Fc(n,g,h);break;case 5:var x=r.stateNode;r.flags&32&&(ai(x,""),r.flags&=-33);var M=xp(n);Fc(n,M,x);break;case 3:case 4:var H=r.stateNode.containerInfo,se=xp(n);wp(n,se,H);break;default:throw Error(a(161))}}catch(fe){pt(n,n.return,fe)}n.flags&=-3}i&4096&&(n.flags&=-4097)}function I_(n){if(n.subtreeFlags&1024)for(n=n.child;n!==null;){var i=n;I_(i),i.tag===5&&i.flags&1024&&i.stateNode.reset(),n=n.sibling}}function yi(n,i){if(i.subtreeFlags&8772)for(i=i.child;i!==null;)R_(n,i.alternate,i),i=i.sibling}function Va(n){for(n=n.child;n!==null;){var i=n;switch(i.tag){case 0:case 11:case 14:case 15:Fi(4,i,i.return),Va(i);break;case 1:Ys(i,i.return);var r=i.stateNode;typeof r.componentWillUnmount=="function"&&O_(i,i.return,r),Va(i);break;case 27:gl(i.stateNode);case 26:case 5:Ys(i,i.return),Va(i);break;case 22:i.memoizedState===null&&Va(i);break;case 30:Va(i);break;default:Va(i)}n=n.sibling}}function _i(n,i,r){for(r=r&&(i.subtreeFlags&8772)!==0,i=i.child;i!==null;){var u=i.alternate,h=n,g=i,x=g.flags;switch(g.tag){case 0:case 11:case 15:_i(h,g,r),ol(4,g);break;case 1:if(_i(h,g,r),u=g,h=u.stateNode,typeof h.componentDidMount=="function")try{h.componentDidMount()}catch(se){pt(u,u.return,se)}if(u=g,h=u.updateQueue,h!==null){var M=u.stateNode;try{var H=h.shared.hiddenCallbacks;if(H!==null)for(h.shared.hiddenCallbacks=null,h=0;h<H.length;h++)by(H[h],M)}catch(se){pt(u,u.return,se)}}r&&x&64&&A_(g),rl(g,g.return);break;case 27:D_(g);case 26:case 5:_i(h,g,r),r&&u===null&&x&4&&M_(g),rl(g,g.return);break;case 12:_i(h,g,r);break;case 31:_i(h,g,r),r&&x&4&&U_(h,g);break;case 13:_i(h,g,r),r&&x&4&&B_(h,g);break;case 22:g.memoizedState===null&&_i(h,g,r),rl(g,g.return);break;case 30:break;default:_i(h,g,r)}i=i.sibling}}function kp(n,i){var r=null;n!==null&&n.memoizedState!==null&&n.memoizedState.cachePool!==null&&(r=n.memoizedState.cachePool.pool),n=null,i.memoizedState!==null&&i.memoizedState.cachePool!==null&&(n=i.memoizedState.cachePool.pool),n!==r&&(n!=null&&n.refCount++,r!=null&&Kr(r))}function jp(n,i){n=null,i.alternate!==null&&(n=i.alternate.memoizedState.cache),i=i.memoizedState.cache,i!==n&&(i.refCount++,n!=null&&Kr(n))}function Es(n,i,r,u){if(i.subtreeFlags&10256)for(i=i.child;i!==null;)Z_(n,i,r,u),i=i.sibling}function Z_(n,i,r,u){var h=i.flags;switch(i.tag){case 0:case 11:case 15:Es(n,i,r,u),h&2048&&ol(9,i);break;case 1:Es(n,i,r,u);break;case 3:Es(n,i,r,u),h&2048&&(n=null,i.alternate!==null&&(n=i.alternate.memoizedState.cache),i=i.memoizedState.cache,i!==n&&(i.refCount++,n!=null&&Kr(n)));break;case 12:if(h&2048){Es(n,i,r,u),n=i.stateNode;try{var g=i.memoizedProps,x=g.id,M=g.onPostCommit;typeof M=="function"&&M(x,i.alternate===null?"mount":"update",n.passiveEffectDuration,-0)}catch(H){pt(i,i.return,H)}}else Es(n,i,r,u);break;case 31:Es(n,i,r,u);break;case 13:Es(n,i,r,u);break;case 23:break;case 22:g=i.stateNode,x=i.alternate,i.memoizedState!==null?g._visibility&2?Es(n,i,r,u):ll(n,i):g._visibility&2?Es(n,i,r,u):(g._visibility|=2,Ho(n,i,r,u,(i.subtreeFlags&10256)!==0||!1)),h&2048&&kp(x,i);break;case 24:Es(n,i,r,u),h&2048&&jp(i.alternate,i);break;default:Es(n,i,r,u)}}function Ho(n,i,r,u,h){for(h=h&&((i.subtreeFlags&10256)!==0||!1),i=i.child;i!==null;){var g=n,x=i,M=r,H=u,se=x.flags;switch(x.tag){case 0:case 11:case 15:Ho(g,x,M,H,h),ol(8,x);break;case 23:break;case 22:var fe=x.stateNode;x.memoizedState!==null?fe._visibility&2?Ho(g,x,M,H,h):ll(g,x):(fe._visibility|=2,Ho(g,x,M,H,h)),h&&se&2048&&kp(x.alternate,x);break;case 24:Ho(g,x,M,H,h),h&&se&2048&&jp(x.alternate,x);break;default:Ho(g,x,M,H,h)}i=i.sibling}}function ll(n,i){if(i.subtreeFlags&10256)for(i=i.child;i!==null;){var r=n,u=i,h=u.flags;switch(u.tag){case 22:ll(r,u),h&2048&&kp(u.alternate,u);break;case 24:ll(r,u),h&2048&&jp(u.alternate,u);break;default:ll(r,u)}i=i.sibling}}var cl=8192;function Io(n,i,r){if(n.subtreeFlags&cl)for(n=n.child;n!==null;)P_(n,i,r),n=n.sibling}function P_(n,i,r){switch(n.tag){case 26:Io(n,i,r),n.flags&cl&&n.memoizedState!==null&&bN(r,Ns,n.memoizedState,n.memoizedProps);break;case 5:Io(n,i,r);break;case 3:case 4:var u=Ns;Ns=du(n.stateNode.containerInfo),Io(n,i,r),Ns=u;break;case 22:n.memoizedState===null&&(u=n.alternate,u!==null&&u.memoizedState!==null?(u=cl,cl=16777216,Io(n,i,r),cl=u):Io(n,i,r));break;default:Io(n,i,r)}}function Y_(n){var i=n.alternate;if(i!==null&&(n=i.child,n!==null)){i.child=null;do i=n.sibling,n.sibling=null,n=i;while(n!==null)}}function ul(n){var i=n.deletions;if((n.flags&16)!==0){if(i!==null)for(var r=0;r<i.length;r++){var u=i[r];fn=u,K_(u,n)}Y_(n)}if(n.subtreeFlags&10256)for(n=n.child;n!==null;)V_(n),n=n.sibling}function V_(n){switch(n.tag){case 0:case 11:case 15:ul(n),n.flags&2048&&Fi(9,n,n.return);break;case 3:ul(n);break;case 12:ul(n);break;case 22:var i=n.stateNode;n.memoizedState!==null&&i._visibility&2&&(n.return===null||n.return.tag!==13)?(i._visibility&=-3,Qc(n)):ul(n);break;default:ul(n)}}function Qc(n){var i=n.deletions;if((n.flags&16)!==0){if(i!==null)for(var r=0;r<i.length;r++){var u=i[r];fn=u,K_(u,n)}Y_(n)}for(n=n.child;n!==null;){switch(i=n,i.tag){case 0:case 11:case 15:Fi(8,i,i.return),Qc(i);break;case 22:r=i.stateNode,r._visibility&2&&(r._visibility&=-3,Qc(i));break;default:Qc(i)}n=n.sibling}}function K_(n,i){for(;fn!==null;){var r=fn;switch(r.tag){case 0:case 11:case 15:Fi(8,r,i);break;case 23:case 22:if(r.memoizedState!==null&&r.memoizedState.cachePool!==null){var u=r.memoizedState.cachePool.pool;u!=null&&u.refCount++}break;case 24:Kr(r.memoizedState.cache)}if(u=r.child,u!==null)u.return=r,fn=u;else e:for(r=n;fn!==null;){u=fn;var h=u.sibling,g=u.return;if(L_(u),u===r){fn=null;break e}if(h!==null){h.return=g,fn=h;break e}fn=g}}}var Dj={getCacheForType:function(n){var i=bn(Wt),r=i.data.get(n);return r===void 0&&(r=n(),i.data.set(n,r)),r},cacheSignal:function(){return bn(Wt).controller.signal}},$j=typeof WeakMap=="function"?WeakMap:Map,ut=0,_t=null,Xe=null,Qe=0,ft=0,Qn=null,Ji=!1,Zo=!1,Np=!1,bi=0,Zt=0,Qi=0,Ka=0,Ep=0,Wn=0,Po=0,dl=null,Bn=null,Tp=!1,Wc=0,G_=0,eu=1/0,tu=null,Wi=null,rn=0,ea=null,Yo=null,vi=0,Cp=0,Ap=null,X_=null,fl=0,Op=null;function es(){return(ut&2)!==0&&Qe!==0?Qe&-Qe:D.T!==null?Lp():wa()}function F_(){if(Wn===0)if((Qe&536870912)===0||et){var n=Re;Re<<=1,(Re&3932160)===0&&(Re=262144),Wn=n}else Wn=536870912;return n=Fn.current,n!==null&&(n.flags|=32),Wn}function Hn(n,i,r){(n===_t&&(ft===2||ft===9)||n.cancelPendingCommit!==null)&&(Vo(n,0),ta(n,Qe,Wn,!1)),Nn(n,r),((ut&2)===0||n!==_t)&&(n===_t&&((ut&2)===0&&(Ka|=r),Zt===4&&ta(n,Qe,Wn,!1)),Vs(n))}function J_(n,i,r){if((ut&6)!==0)throw Error(a(327));var u=!r&&(i&127)===0&&(i&n.expiredLanes)===0||wt(n,i),h=u?qj(n,i):zp(n,i,!0),g=u;do{if(h===0){Zo&&!u&&ta(n,i,0,!1);break}else{if(r=n.current.alternate,g&&!Rj(r)){h=zp(n,i,!1),g=!1;continue}if(h===2){if(g=i,n.errorRecoveryDisabledLanes&g)var x=0;else x=n.pendingLanes&-536870913,x=x!==0?x:x&536870912?536870912:0;if(x!==0){i=x;e:{var M=n;h=dl;var H=M.current.memoizedState.isDehydrated;if(H&&(Vo(M,x).flags|=256),x=zp(M,x,!1),x!==2){if(Np&&!H){M.errorRecoveryDisabledLanes|=g,Ka|=g,h=4;break e}g=Bn,Bn=h,g!==null&&(Bn===null?Bn=g:Bn.push.apply(Bn,g))}h=x}if(g=!1,h!==2)continue}}if(h===1){Vo(n,0),ta(n,i,0,!0);break}e:{switch(u=n,g=h,g){case 0:case 1:throw Error(a(345));case 4:if((i&4194048)!==i)break;case 6:ta(u,i,Wn,!Ji);break e;case 2:Bn=null;break;case 3:case 5:break;default:throw Error(a(329))}if((i&62914560)===i&&(h=Wc+300-lt(),10<h)){if(ta(u,i,Wn,!Ji),ct(u,0,!0)!==0)break e;vi=i,u.timeoutHandle=Tb(Q_.bind(null,u,r,Bn,tu,Tp,i,Wn,Ka,Po,Ji,g,"Throttled",-0,0),h);break e}Q_(u,r,Bn,tu,Tp,i,Wn,Ka,Po,Ji,g,null,-0,0)}}break}while(!0);Vs(n)}function Q_(n,i,r,u,h,g,x,M,H,se,fe,ye,ie,re){if(n.timeoutHandle=-1,ye=i.subtreeFlags,ye&8192||(ye&16785408)===16785408){ye={stylesheets:null,count:0,imgCount:0,imgBytes:0,suspenseyImages:[],waitingForImages:!0,waitingForViewTransition:!1,unsuspend:be},P_(i,g,ye);var Ee=(g&62914560)===g?Wc-lt():(g&4194048)===g?G_-lt():0;if(Ee=vN(ye,Ee),Ee!==null){vi=g,n.cancelPendingCommit=Ee(ob.bind(null,n,i,g,r,u,h,x,M,H,fe,ye,null,ie,re)),ta(n,g,x,!se);return}}ob(n,i,g,r,u,h,x,M,H)}function Rj(n){for(var i=n;;){var r=i.tag;if((r===0||r===11||r===15)&&i.flags&16384&&(r=i.updateQueue,r!==null&&(r=r.stores,r!==null)))for(var u=0;u<r.length;u++){var h=r[u],g=h.getSnapshot;h=h.value;try{if(!Gn(g(),h))return!1}catch{return!1}}if(r=i.child,i.subtreeFlags&16384&&r!==null)r.return=i,i=r;else{if(i===n)break;for(;i.sibling===null;){if(i.return===null||i.return===n)return!0;i=i.return}i.sibling.return=i.return,i=i.sibling}}return!0}function ta(n,i,r,u){i&=~Ep,i&=~Ka,n.suspendedLanes|=i,n.pingedLanes&=~i,u&&(n.warmLanes|=i),u=n.expirationTimes;for(var h=i;0<h;){var g=31-Mt(h),x=1<<g;u[g]=-1,h&=~x}r!==0&&ei(n,r,i)}function nu(){return(ut&6)===0?(pl(0),!1):!0}function Mp(){if(Xe!==null){if(ft===0)var n=Xe.return;else n=Xe,ci=qa=null,Gf(n),Ro=null,Xr=0,n=Xe;for(;n!==null;)C_(n.alternate,n),n=n.return;Xe=null}}function Vo(n,i){var r=n.timeoutHandle;r!==-1&&(n.timeoutHandle=-1,nN(r)),r=n.cancelPendingCommit,r!==null&&(n.cancelPendingCommit=null,r()),vi=0,Mp(),_t=n,Xe=r=ri(n.current,null),Qe=i,ft=0,Qn=null,Ji=!1,Zo=wt(n,i),Np=!1,Po=Wn=Ep=Ka=Qi=Zt=0,Bn=dl=null,Tp=!1,(i&8)!==0&&(i|=i&32);var u=n.entangledLanes;if(u!==0)for(n=n.entanglements,u&=i;0<u;){var h=31-Mt(u),g=1<<h;i|=n[h],u&=~g}return bi=i,Sc(),r}function W_(n,i){Pe=null,D.H=sl,i===$o||i===Oc?(i=hy(),ft=3):i===Rf?(i=hy(),ft=4):ft=i===up?8:i!==null&&typeof i=="object"&&typeof i.then=="function"?6:1,Qn=i,Xe===null&&(Zt=1,Yc(n,cs(i,n.current)))}function eb(){var n=Fn.current;return n===null?!0:(Qe&4194048)===Qe?ps===null:(Qe&62914560)===Qe||(Qe&536870912)!==0?n===ps:!1}function tb(){var n=D.H;return D.H=sl,n===null?sl:n}function nb(){var n=D.A;return D.A=Dj,n}function su(){Zt=4,Ji||(Qe&4194048)!==Qe&&Fn.current!==null||(Zo=!0),(Qi&134217727)===0&&(Ka&134217727)===0||_t===null||ta(_t,Qe,Wn,!1)}function zp(n,i,r){var u=ut;ut|=2;var h=tb(),g=nb();(_t!==n||Qe!==i)&&(tu=null,Vo(n,i)),i=!1;var x=Zt;e:do try{if(ft!==0&&Xe!==null){var M=Xe,H=Qn;switch(ft){case 8:Mp(),x=6;break e;case 3:case 2:case 9:case 6:Fn.current===null&&(i=!0);var se=ft;if(ft=0,Qn=null,Ko(n,M,H,se),r&&Zo){x=0;break e}break;default:se=ft,ft=0,Qn=null,Ko(n,M,H,se)}}Lj(),x=Zt;break}catch(fe){W_(n,fe)}while(!0);return i&&n.shellSuspendCounter++,ci=qa=null,ut=u,D.H=h,D.A=g,Xe===null&&(_t=null,Qe=0,Sc()),x}function Lj(){for(;Xe!==null;)sb(Xe)}function qj(n,i){var r=ut;ut|=2;var u=tb(),h=nb();_t!==n||Qe!==i?(tu=null,eu=lt()+500,Vo(n,i)):Zo=wt(n,i);e:do try{if(ft!==0&&Xe!==null){i=Xe;var g=Qn;t:switch(ft){case 1:ft=0,Qn=null,Ko(n,i,g,1);break;case 2:case 9:if(py(g)){ft=0,Qn=null,ib(i);break}i=function(){ft!==2&&ft!==9||_t!==n||(ft=7),Vs(n)},g.then(i,i);break e;case 3:ft=7;break e;case 4:ft=5;break e;case 7:py(g)?(ft=0,Qn=null,ib(i)):(ft=0,Qn=null,Ko(n,i,g,7));break;case 5:var x=null;switch(Xe.tag){case 26:x=Xe.memoizedState;case 5:case 27:var M=Xe;if(x?Zb(x):M.stateNode.complete){ft=0,Qn=null;var H=M.sibling;if(H!==null)Xe=H;else{var se=M.return;se!==null?(Xe=se,iu(se)):Xe=null}break t}}ft=0,Qn=null,Ko(n,i,g,5);break;case 6:ft=0,Qn=null,Ko(n,i,g,6);break;case 8:Mp(),Zt=6;break e;default:throw Error(a(462))}}Uj();break}catch(fe){W_(n,fe)}while(!0);return ci=qa=null,D.H=u,D.A=h,ut=r,Xe!==null?0:(_t=null,Qe=0,Sc(),Zt)}function Uj(){for(;Xe!==null&&!Xt();)sb(Xe)}function sb(n){var i=E_(n.alternate,n,bi);n.memoizedProps=n.pendingProps,i===null?iu(n):Xe=i}function ib(n){var i=n,r=i.alternate;switch(i.tag){case 15:case 0:i=x_(r,i,i.pendingProps,i.type,void 0,Qe);break;case 11:i=x_(r,i,i.pendingProps,i.type.render,i.ref,Qe);break;case 5:Gf(i);default:C_(r,i),i=Xe=ny(i,bi),i=E_(r,i,bi)}n.memoizedProps=n.pendingProps,i===null?iu(n):Xe=i}function Ko(n,i,r,u){ci=qa=null,Gf(i),Ro=null,Xr=0;var h=i.return;try{if(Ej(n,h,i,r,Qe)){Zt=1,Yc(n,cs(r,n.current)),Xe=null;return}}catch(g){if(h!==null)throw Xe=h,g;Zt=1,Yc(n,cs(r,n.current)),Xe=null;return}i.flags&32768?(et||u===1?n=!0:Zo||(Qe&536870912)!==0?n=!1:(Ji=n=!0,(u===2||u===9||u===3||u===6)&&(u=Fn.current,u!==null&&u.tag===13&&(u.flags|=16384))),ab(i,n)):iu(i)}function iu(n){var i=n;do{if((i.flags&32768)!==0){ab(i,Ji);return}n=i.return;var r=Aj(i.alternate,i,bi);if(r!==null){Xe=r;return}if(i=i.sibling,i!==null){Xe=i;return}Xe=i=n}while(i!==null);Zt===0&&(Zt=5)}function ab(n,i){do{var r=Oj(n.alternate,n);if(r!==null){r.flags&=32767,Xe=r;return}if(r=n.return,r!==null&&(r.flags|=32768,r.subtreeFlags=0,r.deletions=null),!i&&(n=n.sibling,n!==null)){Xe=n;return}Xe=n=r}while(n!==null);Zt=6,Xe=null}function ob(n,i,r,u,h,g,x,M,H){n.cancelPendingCommit=null;do au();while(rn!==0);if((ut&6)!==0)throw Error(a(327));if(i!==null){if(i===n.current)throw Error(a(177));if(g=i.lanes|i.childLanes,g|=xf,Hs(n,r,g,x,M,H),n===_t&&(Xe=_t=null,Qe=0),Yo=i,ea=n,vi=r,Cp=g,Ap=h,X_=u,(i.subtreeFlags&10256)!==0||(i.flags&10256)!==0?(n.callbackNode=null,n.callbackPriority=0,Zj(Ft,function(){return db(),null})):(n.callbackNode=null,n.callbackPriority=0),u=(i.flags&13878)!==0,(i.subtreeFlags&13878)!==0||u){u=D.T,D.T=null,h=U.p,U.p=2,x=ut,ut|=4;try{Mj(n,i,r)}finally{ut=x,U.p=h,D.T=u}}rn=1,rb(),lb(),cb()}}function rb(){if(rn===1){rn=0;var n=ea,i=Yo,r=(i.flags&13878)!==0;if((i.subtreeFlags&13878)!==0||r){r=D.T,D.T=null;var u=U.p;U.p=2;var h=ut;ut|=4;try{H_(i,n);var g=Yp,x=Kg(n.containerInfo),M=g.focusedElem,H=g.selectionRange;if(x!==M&&M&&M.ownerDocument&&Vg(M.ownerDocument.documentElement,M)){if(H!==null&&gf(M)){var se=H.start,fe=H.end;if(fe===void 0&&(fe=se),"selectionStart"in M)M.selectionStart=se,M.selectionEnd=Math.min(fe,M.value.length);else{var ye=M.ownerDocument||document,ie=ye&&ye.defaultView||window;if(ie.getSelection){var re=ie.getSelection(),Ee=M.textContent.length,Le=Math.min(H.start,Ee),gt=H.end===void 0?Le:Math.min(H.end,Ee);!re.extend&&Le>gt&&(x=gt,gt=Le,Le=x);var J=Yg(M,Le),K=Yg(M,gt);if(J&&K&&(re.rangeCount!==1||re.anchorNode!==J.node||re.anchorOffset!==J.offset||re.focusNode!==K.node||re.focusOffset!==K.offset)){var ne=ye.createRange();ne.setStart(J.node,J.offset),re.removeAllRanges(),Le>gt?(re.addRange(ne),re.extend(K.node,K.offset)):(ne.setEnd(K.node,K.offset),re.addRange(ne))}}}}for(ye=[],re=M;re=re.parentNode;)re.nodeType===1&&ye.push({element:re,left:re.scrollLeft,top:re.scrollTop});for(typeof M.focus=="function"&&M.focus(),M=0;M<ye.length;M++){var he=ye[M];he.element.scrollLeft=he.left,he.element.scrollTop=he.top}}yu=!!Pp,Yp=Pp=null}finally{ut=h,U.p=u,D.T=r}}n.current=i,rn=2}}function lb(){if(rn===2){rn=0;var n=ea,i=Yo,r=(i.flags&8772)!==0;if((i.subtreeFlags&8772)!==0||r){r=D.T,D.T=null;var u=U.p;U.p=2;var h=ut;ut|=4;try{R_(n,i.alternate,i)}finally{ut=h,U.p=u,D.T=r}}rn=3}}function cb(){if(rn===4||rn===3){rn=0,Pt();var n=ea,i=Yo,r=vi,u=X_;(i.subtreeFlags&10256)!==0||(i.flags&10256)!==0?rn=5:(rn=0,Yo=ea=null,ub(n,n.pendingLanes));var h=n.pendingLanes;if(h===0&&(Wi=null),ni(r),i=i.stateNode,Qt&&typeof Qt.onCommitFiberRoot=="function")try{Qt.onCommitFiberRoot(Dn,i,void 0,(i.current.flags&128)===128)}catch{}if(u!==null){i=D.T,h=U.p,U.p=2,D.T=null;try{for(var g=n.onRecoverableError,x=0;x<u.length;x++){var M=u[x];g(M.value,{componentStack:M.stack})}}finally{D.T=i,U.p=h}}(vi&3)!==0&&au(),Vs(n),h=n.pendingLanes,(r&261930)!==0&&(h&42)!==0?n===Op?fl++:(fl=0,Op=n):fl=0,pl(0)}}function ub(n,i){(n.pooledCacheLanes&=i)===0&&(i=n.pooledCache,i!=null&&(n.pooledCache=null,Kr(i)))}function au(){return rb(),lb(),cb(),db()}function db(){if(rn!==5)return!1;var n=ea,i=Cp;Cp=0;var r=ni(vi),u=D.T,h=U.p;try{U.p=32>r?32:r,D.T=null,r=Ap,Ap=null;var g=ea,x=vi;if(rn=0,Yo=ea=null,vi=0,(ut&6)!==0)throw Error(a(331));var M=ut;if(ut|=4,V_(g.current),Z_(g,g.current,x,r),ut=M,pl(0,!1),Qt&&typeof Qt.onPostCommitFiberRoot=="function")try{Qt.onPostCommitFiberRoot(Dn,g)}catch{}return!0}finally{U.p=h,D.T=u,ub(n,i)}}function fb(n,i,r){i=cs(r,i),i=cp(n.stateNode,i,2),n=Ki(n,i,2),n!==null&&(Nn(n,2),Vs(n))}function pt(n,i,r){if(n.tag===3)fb(n,n,r);else for(;i!==null;){if(i.tag===3){fb(i,n,r);break}else if(i.tag===1){var u=i.stateNode;if(typeof i.type.getDerivedStateFromError=="function"||typeof u.componentDidCatch=="function"&&(Wi===null||!Wi.has(u))){n=cs(r,n),r=p_(2),u=Ki(i,r,2),u!==null&&(m_(r,u,i,n),Nn(u,2),Vs(u));break}}i=i.return}}function Dp(n,i,r){var u=n.pingCache;if(u===null){u=n.pingCache=new $j;var h=new Set;u.set(i,h)}else h=u.get(i),h===void 0&&(h=new Set,u.set(i,h));h.has(r)||(Np=!0,h.add(r),n=Bj.bind(null,n,i,r),i.then(n,n))}function Bj(n,i,r){var u=n.pingCache;u!==null&&u.delete(i),n.pingedLanes|=n.suspendedLanes&r,n.warmLanes&=~r,_t===n&&(Qe&r)===r&&(Zt===4||Zt===3&&(Qe&62914560)===Qe&&300>lt()-Wc?(ut&2)===0&&Vo(n,0):Ep|=r,Po===Qe&&(Po=0)),Vs(n)}function pb(n,i){i===0&&(i=Ws()),n=$a(n,i),n!==null&&(Nn(n,i),Vs(n))}function Hj(n){var i=n.memoizedState,r=0;i!==null&&(r=i.retryLane),pb(n,r)}function Ij(n,i){var r=0;switch(n.tag){case 31:case 13:var u=n.stateNode,h=n.memoizedState;h!==null&&(r=h.retryLane);break;case 19:u=n.stateNode;break;case 22:u=n.stateNode._retryCache;break;default:throw Error(a(314))}u!==null&&u.delete(i),pb(n,r)}function Zj(n,i){return nt(n,i)}var ou=null,Go=null,$p=!1,ru=!1,Rp=!1,na=0;function Vs(n){n!==Go&&n.next===null&&(Go===null?ou=Go=n:Go=Go.next=n),ru=!0,$p||($p=!0,Yj())}function pl(n,i){if(!Rp&&ru){Rp=!0;do for(var r=!1,u=ou;u!==null;){if(n!==0){var h=u.pendingLanes;if(h===0)var g=0;else{var x=u.suspendedLanes,M=u.pingedLanes;g=(1<<31-Mt(42|n)+1)-1,g&=h&~(x&~M),g=g&201326741?g&201326741|1:g?g|2:0}g!==0&&(r=!0,yb(u,g))}else g=Qe,g=ct(u,u===_t?g:0,u.cancelPendingCommit!==null||u.timeoutHandle!==-1),(g&3)===0||wt(u,g)||(r=!0,yb(u,g));u=u.next}while(r);Rp=!1}}function Pj(){mb()}function mb(){ru=$p=!1;var n=0;na!==0&&tN()&&(n=na);for(var i=lt(),r=null,u=ou;u!==null;){var h=u.next,g=hb(u,i);g===0?(u.next=null,r===null?ou=h:r.next=h,h===null&&(Go=r)):(r=u,(n!==0||(g&3)!==0)&&(ru=!0)),u=h}rn!==0&&rn!==5||pl(n),na!==0&&(na=0)}function hb(n,i){for(var r=n.suspendedLanes,u=n.pingedLanes,h=n.expirationTimes,g=n.pendingLanes&-62914561;0<g;){var x=31-Mt(g),M=1<<x,H=h[x];H===-1?((M&r)===0||(M&u)!==0)&&(h[x]=Vn(M,i)):H<=i&&(n.expiredLanes|=M),g&=~M}if(i=_t,r=Qe,r=ct(n,n===i?r:0,n.cancelPendingCommit!==null||n.timeoutHandle!==-1),u=n.callbackNode,r===0||n===i&&(ft===2||ft===9)||n.cancelPendingCommit!==null)return u!==null&&u!==null&&qe(u),n.callbackNode=null,n.callbackPriority=0;if((r&3)===0||wt(n,r)){if(i=r&-r,i===n.callbackPriority)return i;switch(u!==null&&qe(u),ni(r)){case 2:case 8:r=Tt;break;case 32:r=Ft;break;case 268435456:r=zn;break;default:r=Ft}return u=gb.bind(null,n),r=nt(r,u),n.callbackPriority=i,n.callbackNode=r,i}return u!==null&&u!==null&&qe(u),n.callbackPriority=2,n.callbackNode=null,2}function gb(n,i){if(rn!==0&&rn!==5)return n.callbackNode=null,n.callbackPriority=0,null;var r=n.callbackNode;if(au()&&n.callbackNode!==r)return null;var u=Qe;return u=ct(n,n===_t?u:0,n.cancelPendingCommit!==null||n.timeoutHandle!==-1),u===0?null:(J_(n,u,i),hb(n,lt()),n.callbackNode!=null&&n.callbackNode===r?gb.bind(null,n):null)}function yb(n,i){if(au())return null;J_(n,i,!0)}function Yj(){sN(function(){(ut&6)!==0?nt(mn,Pj):mb()})}function Lp(){if(na===0){var n=zo;n===0&&(n=Te,Te<<=1,(Te&261888)===0&&(Te=256)),na=n}return na}function _b(n){return n==null||typeof n=="symbol"||typeof n=="boolean"?null:typeof n=="function"?n:pe(""+n)}function bb(n,i){var r=i.ownerDocument.createElement("input");return r.name=i.name,r.value=i.value,n.id&&r.setAttribute("form",n.id),i.parentNode.insertBefore(r,i),n=new FormData(n),r.parentNode.removeChild(r),n}function Vj(n,i,r,u,h){if(i==="submit"&&r&&r.stateNode===h){var g=_b((h[Yt]||null).action),x=u.submitter;x&&(i=(i=x[Yt]||null)?_b(i.formAction):x.getAttribute("formAction"),i!==null&&(g=i,x=null));var M=new bc("action","action",null,u,h);n.push({event:M,listeners:[{instance:null,listener:function(){if(u.defaultPrevented){if(na!==0){var H=x?bb(h,x):new FormData(h);sp(r,{pending:!0,data:H,method:h.method,action:g},null,H)}}else typeof g=="function"&&(M.preventDefault(),H=x?bb(h,x):new FormData(h),sp(r,{pending:!0,data:H,method:h.method,action:g},g,H))},currentTarget:h}]})}}for(var qp=0;qp<vf.length;qp++){var Up=vf[qp],Kj=Up.toLowerCase(),Gj=Up[0].toUpperCase()+Up.slice(1);js(Kj,"on"+Gj)}js(Fg,"onAnimationEnd"),js(Jg,"onAnimationIteration"),js(Qg,"onAnimationStart"),js("dblclick","onDoubleClick"),js("focusin","onFocus"),js("focusout","onBlur"),js(uj,"onTransitionRun"),js(dj,"onTransitionStart"),js(fj,"onTransitionCancel"),js(Wg,"onTransitionEnd"),rs("onMouseEnter",["mouseout","mouseover"]),rs("onMouseLeave",["mouseout","mouseover"]),rs("onPointerEnter",["pointerout","pointerover"]),rs("onPointerLeave",["pointerout","pointerover"]),os("onChange","change click focusin focusout input keydown keyup selectionchange".split(" ")),os("onSelect","focusout contextmenu dragend focusin keydown keyup mousedown mouseup selectionchange".split(" ")),os("onBeforeInput",["compositionend","keypress","textInput","paste"]),os("onCompositionEnd","compositionend focusout keydown keypress keyup mousedown".split(" ")),os("onCompositionStart","compositionstart focusout keydown keypress keyup mousedown".split(" ")),os("onCompositionUpdate","compositionupdate focusout keydown keypress keyup mousedown".split(" "));var ml="abort canplay canplaythrough durationchange emptied encrypted ended error loadeddata loadedmetadata loadstart pause play playing progress ratechange resize seeked seeking stalled suspend timeupdate volumechange waiting".split(" "),Xj=new Set("beforetoggle cancel close invalid load scroll scrollend toggle".split(" ").concat(ml));function vb(n,i){i=(i&4)!==0;for(var r=0;r<n.length;r++){var u=n[r],h=u.event;u=u.listeners;e:{var g=void 0;if(i)for(var x=u.length-1;0<=x;x--){var M=u[x],H=M.instance,se=M.currentTarget;if(M=M.listener,H!==g&&h.isPropagationStopped())break e;g=M,h.currentTarget=se;try{g(h)}catch(fe){wc(fe)}h.currentTarget=null,g=H}else for(x=0;x<u.length;x++){if(M=u[x],H=M.instance,se=M.currentTarget,M=M.listener,H!==g&&h.isPropagationStopped())break e;g=M,h.currentTarget=se;try{g(h)}catch(fe){wc(fe)}h.currentTarget=null,g=H}}}}function Fe(n,i){var r=i[xs];r===void 0&&(r=i[xs]=new Set);var u=n+"__bubble";r.has(u)||(xb(i,n,2,!1),r.add(u))}function Bp(n,i,r){var u=0;i&&(u|=4),xb(r,n,u,i)}var lu="_reactListening"+Math.random().toString(36).slice(2);function Hp(n){if(!n[lu]){n[lu]=!0,bo.forEach(function(r){r!=="selectionchange"&&(Xj.has(r)||Bp(r,!1,n),Bp(r,!0,n))});var i=n.nodeType===9?n:n.ownerDocument;i===null||i[lu]||(i[lu]=!0,Bp("selectionchange",!1,i))}}function xb(n,i,r,u){switch(Fb(i)){case 2:var h=SN;break;case 8:h=kN;break;default:h=nm}r=h.bind(null,i,r,n),h=void 0,!So||i!=="touchstart"&&i!=="touchmove"&&i!=="wheel"||(h=!0),u?h!==void 0?n.addEventListener(i,r,{capture:!0,passive:h}):n.addEventListener(i,r,!0):h!==void 0?n.addEventListener(i,r,{passive:h}):n.addEventListener(i,r,!1)}function Ip(n,i,r,u,h){var g=u;if((i&1)===0&&(i&2)===0&&u!==null)e:for(;;){if(u===null)return;var x=u.tag;if(x===3||x===4){var M=u.stateNode.containerInfo;if(M===h)break;if(x===4)for(x=u.return;x!==null;){var H=x.tag;if((H===3||H===4)&&x.stateNode.containerInfo===h)return;x=x.return}for(;M!==null;){if(x=un(M),x===null)return;if(H=x.tag,H===5||H===6||H===26||H===27){u=g=x;continue e}M=M.parentNode}}u=u.return}oi(function(){var se=g,fe=dn(r),ye=[];e:{var ie=ey.get(n);if(ie!==void 0){var re=bc,Ee=n;switch(n){case"keypress":if(yc(r)===0)break e;case"keydown":case"keyup":re=Ik;break;case"focusin":Ee="focus",re=df;break;case"focusout":Ee="blur",re=df;break;case"beforeblur":case"afterblur":re=df;break;case"click":if(r.button===2)break e;case"auxclick":case"dblclick":case"mousedown":case"mousemove":case"mouseup":case"mouseout":case"mouseover":case"contextmenu":re=Cg;break;case"drag":case"dragend":case"dragenter":case"dragexit":case"dragleave":case"dragover":case"dragstart":case"drop":re=Ak;break;case"touchcancel":case"touchend":case"touchmove":case"touchstart":re=Yk;break;case Fg:case Jg:case Qg:re=zk;break;case Wg:re=Kk;break;case"scroll":case"scrollend":re=Tk;break;case"wheel":re=Xk;break;case"copy":case"cut":case"paste":re=$k;break;case"gotpointercapture":case"lostpointercapture":case"pointercancel":case"pointerdown":case"pointermove":case"pointerout":case"pointerover":case"pointerup":re=Og;break;case"toggle":case"beforetoggle":re=Jk}var Le=(i&4)!==0,gt=!Le&&(n==="scroll"||n==="scrollend"),J=Le?ie!==null?ie+"Capture":null:ie;Le=[];for(var K=se,ne;K!==null;){var he=K;if(ne=he.stateNode,he=he.tag,he!==5&&he!==26&&he!==27||ne===null||J===null||(he=Ht(K,J),he!=null&&Le.push(hl(K,he,ne))),gt)break;K=K.return}0<Le.length&&(ie=new re(ie,Ee,null,r,fe),ye.push({event:ie,listeners:Le}))}}if((i&7)===0){e:{if(ie=n==="mouseover"||n==="pointerover",re=n==="mouseout"||n==="pointerout",ie&&r!==Oe&&(Ee=r.relatedTarget||r.fromElement)&&(un(Ee)||Ee[as]))break e;if((re||ie)&&(ie=fe.window===fe?fe:(ie=fe.ownerDocument)?ie.defaultView||ie.parentWindow:window,re?(Ee=r.relatedTarget||r.toElement,re=se,Ee=Ee?un(Ee):null,Ee!==null&&(gt=l(Ee),Le=Ee.tag,Ee!==gt||Le!==5&&Le!==27&&Le!==6)&&(Ee=null)):(re=null,Ee=se),re!==Ee)){if(Le=Cg,he="onMouseLeave",J="onMouseEnter",K="mouse",(n==="pointerout"||n==="pointerover")&&(Le=Og,he="onPointerLeave",J="onPointerEnter",K="pointer"),gt=re==null?ie:zt(re),ne=Ee==null?ie:zt(Ee),ie=new Le(he,K+"leave",re,r,fe),ie.target=gt,ie.relatedTarget=ne,he=null,un(fe)===se&&(Le=new Le(J,K+"enter",Ee,r,fe),Le.target=ne,Le.relatedTarget=gt,he=Le),gt=he,re&&Ee)t:{for(Le=Fj,J=re,K=Ee,ne=0,he=J;he;he=Le(he))ne++;he=0;for(var $e=K;$e;$e=Le($e))he++;for(;0<ne-he;)J=Le(J),ne--;for(;0<he-ne;)K=Le(K),he--;for(;ne--;){if(J===K||K!==null&&J===K.alternate){Le=J;break t}J=Le(J),K=Le(K)}Le=null}else Le=null;re!==null&&wb(ye,ie,re,Le,!1),Ee!==null&&gt!==null&&wb(ye,gt,Ee,Le,!0)}}e:{if(ie=se?zt(se):window,re=ie.nodeName&&ie.nodeName.toLowerCase(),re==="select"||re==="input"&&ie.type==="file")var at=Ug;else if(Lg(ie))if(Bg)at=rj;else{at=aj;var Ae=ij}else re=ie.nodeName,!re||re.toLowerCase()!=="input"||ie.type!=="checkbox"&&ie.type!=="radio"?se&&Aa(se.elementType)&&(at=Ug):at=oj;if(at&&(at=at(n,se))){qg(ye,at,r,fe);break e}Ae&&Ae(n,ie,se),n==="focusout"&&se&&ie.type==="number"&&se.memoizedProps.value!=null&&Ca(ie,"number",ie.value)}switch(Ae=se?zt(se):window,n){case"focusin":(Lg(Ae)||Ae.contentEditable==="true")&&(jo=Ae,yf=se,Pr=null);break;case"focusout":Pr=yf=jo=null;break;case"mousedown":_f=!0;break;case"contextmenu":case"mouseup":case"dragend":_f=!1,Gg(ye,r,fe);break;case"selectionchange":if(cj)break;case"keydown":case"keyup":Gg(ye,r,fe)}var Ye;if(pf)e:{switch(n){case"compositionstart":var We="onCompositionStart";break e;case"compositionend":We="onCompositionEnd";break e;case"compositionupdate":We="onCompositionUpdate";break e}We=void 0}else ko?$g(n,r)&&(We="onCompositionEnd"):n==="keydown"&&r.keyCode===229&&(We="onCompositionStart");We&&(Mg&&r.locale!=="ko"&&(ko||We!=="onCompositionStart"?We==="onCompositionEnd"&&ko&&(Ye=Eg()):(Bi=fe,lf="value"in Bi?Bi.value:Bi.textContent,ko=!0)),Ae=cu(se,We),0<Ae.length&&(We=new Ag(We,n,null,r,fe),ye.push({event:We,listeners:Ae}),Ye?We.data=Ye:(Ye=Rg(r),Ye!==null&&(We.data=Ye)))),(Ye=Wk?ej(n,r):tj(n,r))&&(We=cu(se,"onBeforeInput"),0<We.length&&(Ae=new Ag("onBeforeInput","beforeinput",null,r,fe),ye.push({event:Ae,listeners:We}),Ae.data=Ye)),Vj(ye,n,se,r,fe)}vb(ye,i)})}function hl(n,i,r){return{instance:n,listener:i,currentTarget:r}}function cu(n,i){for(var r=i+"Capture",u=[];n!==null;){var h=n,g=h.stateNode;if(h=h.tag,h!==5&&h!==26&&h!==27||g===null||(h=Ht(n,r),h!=null&&u.unshift(hl(n,h,g)),h=Ht(n,i),h!=null&&u.push(hl(n,h,g))),n.tag===3)return u;n=n.return}return[]}function Fj(n){if(n===null)return null;do n=n.return;while(n&&n.tag!==5&&n.tag!==27);return n||null}function wb(n,i,r,u,h){for(var g=i._reactName,x=[];r!==null&&r!==u;){var M=r,H=M.alternate,se=M.stateNode;if(M=M.tag,H!==null&&H===u)break;M!==5&&M!==26&&M!==27||se===null||(H=se,h?(se=Ht(r,g),se!=null&&x.unshift(hl(r,se,H))):h||(se=Ht(r,g),se!=null&&x.push(hl(r,se,H)))),r=r.return}x.length!==0&&n.push({event:i,listeners:x})}var Jj=/\r\n?/g,Qj=/\u0000|\uFFFD/g;function Sb(n){return(typeof n=="string"?n:""+n).replace(Jj,`
`).replace(Qj,"")}function kb(n,i){return i=Sb(i),Sb(n)===i}function ht(n,i,r,u,h,g){switch(r){case"children":typeof u=="string"?i==="body"||i==="textarea"&&u===""||ai(n,u):(typeof u=="number"||typeof u=="bigint")&&i!=="body"&&ai(n,""+u);break;case"className":si(n,"class",u);break;case"tabIndex":si(n,"tabindex",u);break;case"dir":case"role":case"viewBox":case"width":case"height":si(n,r,u);break;case"style":Lr(n,u,g);break;case"data":if(i!=="object"){si(n,"data",u);break}case"src":case"href":if(u===""&&(i!=="a"||r!=="href")){n.removeAttribute(r);break}if(u==null||typeof u=="function"||typeof u=="symbol"||typeof u=="boolean"){n.removeAttribute(r);break}u=pe(""+u),n.setAttribute(r,u);break;case"action":case"formAction":if(typeof u=="function"){n.setAttribute(r,"javascript:throw new Error('A React form was unexpectedly submitted. If you called form.submit() manually, consider using form.requestSubmit() instead. If you\\'re trying to use event.stopPropagation() in a submit event handler, consider also calling event.preventDefault().')");break}else typeof g=="function"&&(r==="formAction"?(i!=="input"&&ht(n,i,"name",h.name,h,null),ht(n,i,"formEncType",h.formEncType,h,null),ht(n,i,"formMethod",h.formMethod,h,null),ht(n,i,"formTarget",h.formTarget,h,null)):(ht(n,i,"encType",h.encType,h,null),ht(n,i,"method",h.method,h,null),ht(n,i,"target",h.target,h,null)));if(u==null||typeof u=="symbol"||typeof u=="boolean"){n.removeAttribute(r);break}u=pe(""+u),n.setAttribute(r,u);break;case"onClick":u!=null&&(n.onclick=be);break;case"onScroll":u!=null&&Fe("scroll",n);break;case"onScrollEnd":u!=null&&Fe("scrollend",n);break;case"dangerouslySetInnerHTML":if(u!=null){if(typeof u!="object"||!("__html"in u))throw Error(a(61));if(r=u.__html,r!=null){if(h.children!=null)throw Error(a(60));n.innerHTML=r}}break;case"multiple":n.multiple=u&&typeof u!="function"&&typeof u!="symbol";break;case"muted":n.muted=u&&typeof u!="function"&&typeof u!="symbol";break;case"suppressContentEditableWarning":case"suppressHydrationWarning":case"defaultValue":case"defaultChecked":case"innerHTML":case"ref":break;case"autoFocus":break;case"xlinkHref":if(u==null||typeof u=="function"||typeof u=="boolean"||typeof u=="symbol"){n.removeAttribute("xlink:href");break}r=pe(""+u),n.setAttributeNS("http://www.w3.org/1999/xlink","xlink:href",r);break;case"contentEditable":case"spellCheck":case"draggable":case"value":case"autoReverse":case"externalResourcesRequired":case"focusable":case"preserveAlpha":u!=null&&typeof u!="function"&&typeof u!="symbol"?n.setAttribute(r,""+u):n.removeAttribute(r);break;case"inert":case"allowFullScreen":case"async":case"autoPlay":case"controls":case"default":case"defer":case"disabled":case"disablePictureInPicture":case"disableRemotePlayback":case"formNoValidate":case"hidden":case"loop":case"noModule":case"noValidate":case"open":case"playsInline":case"readOnly":case"required":case"reversed":case"scoped":case"seamless":case"itemScope":u&&typeof u!="function"&&typeof u!="symbol"?n.setAttribute(r,""):n.removeAttribute(r);break;case"capture":case"download":u===!0?n.setAttribute(r,""):u!==!1&&u!=null&&typeof u!="function"&&typeof u!="symbol"?n.setAttribute(r,u):n.removeAttribute(r);break;case"cols":case"rows":case"size":case"span":u!=null&&typeof u!="function"&&typeof u!="symbol"&&!isNaN(u)&&1<=u?n.setAttribute(r,u):n.removeAttribute(r);break;case"rowSpan":case"start":u==null||typeof u=="function"||typeof u=="symbol"||isNaN(u)?n.removeAttribute(r):n.setAttribute(r,u);break;case"popover":Fe("beforetoggle",n),Fe("toggle",n),Ss(n,"popover",u);break;case"xlinkActuate":$n(n,"http://www.w3.org/1999/xlink","xlink:actuate",u);break;case"xlinkArcrole":$n(n,"http://www.w3.org/1999/xlink","xlink:arcrole",u);break;case"xlinkRole":$n(n,"http://www.w3.org/1999/xlink","xlink:role",u);break;case"xlinkShow":$n(n,"http://www.w3.org/1999/xlink","xlink:show",u);break;case"xlinkTitle":$n(n,"http://www.w3.org/1999/xlink","xlink:title",u);break;case"xlinkType":$n(n,"http://www.w3.org/1999/xlink","xlink:type",u);break;case"xmlBase":$n(n,"http://www.w3.org/XML/1998/namespace","xml:base",u);break;case"xmlLang":$n(n,"http://www.w3.org/XML/1998/namespace","xml:lang",u);break;case"xmlSpace":$n(n,"http://www.w3.org/XML/1998/namespace","xml:space",u);break;case"is":Ss(n,"is",u);break;case"innerText":case"textContent":break;default:(!(2<r.length)||r[0]!=="o"&&r[0]!=="O"||r[1]!=="n"&&r[1]!=="N")&&(r=$.get(r)||r,Ss(n,r,u))}}function Zp(n,i,r,u,h,g){switch(r){case"style":Lr(n,u,g);break;case"dangerouslySetInnerHTML":if(u!=null){if(typeof u!="object"||!("__html"in u))throw Error(a(61));if(r=u.__html,r!=null){if(h.children!=null)throw Error(a(60));n.innerHTML=r}}break;case"children":typeof u=="string"?ai(n,u):(typeof u=="number"||typeof u=="bigint")&&ai(n,""+u);break;case"onScroll":u!=null&&Fe("scroll",n);break;case"onScrollEnd":u!=null&&Fe("scrollend",n);break;case"onClick":u!=null&&(n.onclick=be);break;case"suppressContentEditableWarning":case"suppressHydrationWarning":case"innerHTML":case"ref":break;case"innerText":case"textContent":break;default:if(!ws.hasOwnProperty(r))e:{if(r[0]==="o"&&r[1]==="n"&&(h=r.endsWith("Capture"),i=r.slice(2,h?r.length-7:void 0),g=n[Yt]||null,g=g!=null?g[r]:null,typeof g=="function"&&n.removeEventListener(i,g,h),typeof u=="function")){typeof g!="function"&&g!==null&&(r in n?n[r]=null:n.hasAttribute(r)&&n.removeAttribute(r)),n.addEventListener(i,u,h);break e}r in n?n[r]=u:u===!0?n.setAttribute(r,""):Ss(n,r,u)}}}function xn(n,i,r){switch(i){case"div":case"span":case"svg":case"path":case"a":case"g":case"p":case"li":break;case"img":Fe("error",n),Fe("load",n);var u=!1,h=!1,g;for(g in r)if(r.hasOwnProperty(g)){var x=r[g];if(x!=null)switch(g){case"src":u=!0;break;case"srcSet":h=!0;break;case"children":case"dangerouslySetInnerHTML":throw Error(a(137,i));default:ht(n,i,g,x,r,null)}}h&&ht(n,i,"srcSet",r.srcSet,r,null),u&&ht(n,i,"src",r.src,r,null);return;case"input":Fe("invalid",n);var M=g=x=h=null,H=null,se=null;for(u in r)if(r.hasOwnProperty(u)){var fe=r[u];if(fe!=null)switch(u){case"name":h=fe;break;case"type":x=fe;break;case"checked":H=fe;break;case"defaultChecked":se=fe;break;case"value":g=fe;break;case"defaultValue":M=fe;break;case"children":case"dangerouslySetInnerHTML":if(fe!=null)throw Error(a(137,i));break;default:ht(n,i,u,fe,r,null)}}qi(n,g,M,H,se,x,h,!1);return;case"select":Fe("invalid",n),u=x=g=null;for(h in r)if(r.hasOwnProperty(h)&&(M=r[h],M!=null))switch(h){case"value":g=M;break;case"defaultValue":x=M;break;case"multiple":u=M;default:ht(n,i,h,M,r,null)}i=g,r=x,n.multiple=!!u,i!=null?Is(n,!!u,i,!1):r!=null&&Is(n,!!u,r,!0);return;case"textarea":Fe("invalid",n),g=h=u=null;for(x in r)if(r.hasOwnProperty(x)&&(M=r[x],M!=null))switch(x){case"value":u=M;break;case"defaultValue":h=M;break;case"children":g=M;break;case"dangerouslySetInnerHTML":if(M!=null)throw Error(a(91));break;default:ht(n,i,x,M,r,null)}pc(n,u,h,g);return;case"option":for(H in r)r.hasOwnProperty(H)&&(u=r[H],u!=null)&&(H==="selected"?n.selected=u&&typeof u!="function"&&typeof u!="symbol":ht(n,i,H,u,r,null));return;case"dialog":Fe("beforetoggle",n),Fe("toggle",n),Fe("cancel",n),Fe("close",n);break;case"iframe":case"object":Fe("load",n);break;case"video":case"audio":for(u=0;u<ml.length;u++)Fe(ml[u],n);break;case"image":Fe("error",n),Fe("load",n);break;case"details":Fe("toggle",n);break;case"embed":case"source":case"link":Fe("error",n),Fe("load",n);case"area":case"base":case"br":case"col":case"hr":case"keygen":case"meta":case"param":case"track":case"wbr":case"menuitem":for(se in r)if(r.hasOwnProperty(se)&&(u=r[se],u!=null))switch(se){case"children":case"dangerouslySetInnerHTML":throw Error(a(137,i));default:ht(n,i,se,u,r,null)}return;default:if(Aa(i)){for(fe in r)r.hasOwnProperty(fe)&&(u=r[fe],u!==void 0&&Zp(n,i,fe,u,r,void 0));return}}for(M in r)r.hasOwnProperty(M)&&(u=r[M],u!=null&&ht(n,i,M,u,r,null))}function Wj(n,i,r,u){switch(i){case"div":case"span":case"svg":case"path":case"a":case"g":case"p":case"li":break;case"input":var h=null,g=null,x=null,M=null,H=null,se=null,fe=null;for(re in r){var ye=r[re];if(r.hasOwnProperty(re)&&ye!=null)switch(re){case"checked":break;case"value":break;case"defaultValue":H=ye;default:u.hasOwnProperty(re)||ht(n,i,re,null,u,ye)}}for(var ie in u){var re=u[ie];if(ye=r[ie],u.hasOwnProperty(ie)&&(re!=null||ye!=null))switch(ie){case"type":g=re;break;case"name":h=re;break;case"checked":se=re;break;case"defaultChecked":fe=re;break;case"value":x=re;break;case"defaultValue":M=re;break;case"children":case"dangerouslySetInnerHTML":if(re!=null)throw Error(a(137,i));break;default:re!==ye&&ht(n,i,ie,re,u,ye)}}ii(n,x,M,H,se,fe,g,h);return;case"select":re=x=M=ie=null;for(g in r)if(H=r[g],r.hasOwnProperty(g)&&H!=null)switch(g){case"value":break;case"multiple":re=H;default:u.hasOwnProperty(g)||ht(n,i,g,null,u,H)}for(h in u)if(g=u[h],H=r[h],u.hasOwnProperty(h)&&(g!=null||H!=null))switch(h){case"value":ie=g;break;case"defaultValue":M=g;break;case"multiple":x=g;default:g!==H&&ht(n,i,h,g,u,H)}i=M,r=x,u=re,ie!=null?Is(n,!!r,ie,!1):!!u!=!!r&&(i!=null?Is(n,!!r,i,!0):Is(n,!!r,r?[]:"",!1));return;case"textarea":re=ie=null;for(M in r)if(h=r[M],r.hasOwnProperty(M)&&h!=null&&!u.hasOwnProperty(M))switch(M){case"value":break;case"children":break;default:ht(n,i,M,null,u,h)}for(x in u)if(h=u[x],g=r[x],u.hasOwnProperty(x)&&(h!=null||g!=null))switch(x){case"value":ie=h;break;case"defaultValue":re=h;break;case"children":break;case"dangerouslySetInnerHTML":if(h!=null)throw Error(a(91));break;default:h!==g&&ht(n,i,x,h,u,g)}Ui(n,ie,re);return;case"option":for(var Ee in r)ie=r[Ee],r.hasOwnProperty(Ee)&&ie!=null&&!u.hasOwnProperty(Ee)&&(Ee==="selected"?n.selected=!1:ht(n,i,Ee,null,u,ie));for(H in u)ie=u[H],re=r[H],u.hasOwnProperty(H)&&ie!==re&&(ie!=null||re!=null)&&(H==="selected"?n.selected=ie&&typeof ie!="function"&&typeof ie!="symbol":ht(n,i,H,ie,u,re));return;case"img":case"link":case"area":case"base":case"br":case"col":case"embed":case"hr":case"keygen":case"meta":case"param":case"source":case"track":case"wbr":case"menuitem":for(var Le in r)ie=r[Le],r.hasOwnProperty(Le)&&ie!=null&&!u.hasOwnProperty(Le)&&ht(n,i,Le,null,u,ie);for(se in u)if(ie=u[se],re=r[se],u.hasOwnProperty(se)&&ie!==re&&(ie!=null||re!=null))switch(se){case"children":case"dangerouslySetInnerHTML":if(ie!=null)throw Error(a(137,i));break;default:ht(n,i,se,ie,u,re)}return;default:if(Aa(i)){for(var gt in r)ie=r[gt],r.hasOwnProperty(gt)&&ie!==void 0&&!u.hasOwnProperty(gt)&&Zp(n,i,gt,void 0,u,ie);for(fe in u)ie=u[fe],re=r[fe],!u.hasOwnProperty(fe)||ie===re||ie===void 0&&re===void 0||Zp(n,i,fe,ie,u,re);return}}for(var J in r)ie=r[J],r.hasOwnProperty(J)&&ie!=null&&!u.hasOwnProperty(J)&&ht(n,i,J,null,u,ie);for(ye in u)ie=u[ye],re=r[ye],!u.hasOwnProperty(ye)||ie===re||ie==null&&re==null||ht(n,i,ye,ie,u,re)}function jb(n){switch(n){case"css":case"script":case"font":case"img":case"image":case"input":case"link":return!0;default:return!1}}function eN(){if(typeof performance.getEntriesByType=="function"){for(var n=0,i=0,r=performance.getEntriesByType("resource"),u=0;u<r.length;u++){var h=r[u],g=h.transferSize,x=h.initiatorType,M=h.duration;if(g&&M&&jb(x)){for(x=0,M=h.responseEnd,u+=1;u<r.length;u++){var H=r[u],se=H.startTime;if(se>M)break;var fe=H.transferSize,ye=H.initiatorType;fe&&jb(ye)&&(H=H.responseEnd,x+=fe*(H<M?1:(M-se)/(H-se)))}if(--u,i+=8*(g+x)/(h.duration/1e3),n++,10<n)break}}if(0<n)return i/n/1e6}return navigator.connection&&(n=navigator.connection.downlink,typeof n=="number")?n:5}var Pp=null,Yp=null;function uu(n){return n.nodeType===9?n:n.ownerDocument}function Nb(n){switch(n){case"http://www.w3.org/2000/svg":return 1;case"http://www.w3.org/1998/Math/MathML":return 2;default:return 0}}function Eb(n,i){if(n===0)switch(i){case"svg":return 1;case"math":return 2;default:return 0}return n===1&&i==="foreignObject"?0:n}function Vp(n,i){return n==="textarea"||n==="noscript"||typeof i.children=="string"||typeof i.children=="number"||typeof i.children=="bigint"||typeof i.dangerouslySetInnerHTML=="object"&&i.dangerouslySetInnerHTML!==null&&i.dangerouslySetInnerHTML.__html!=null}var Kp=null;function tN(){var n=window.event;return n&&n.type==="popstate"?n===Kp?!1:(Kp=n,!0):(Kp=null,!1)}var Tb=typeof setTimeout=="function"?setTimeout:void 0,nN=typeof clearTimeout=="function"?clearTimeout:void 0,Cb=typeof Promise=="function"?Promise:void 0,sN=typeof queueMicrotask=="function"?queueMicrotask:typeof Cb<"u"?function(n){return Cb.resolve(null).then(n).catch(iN)}:Tb;function iN(n){setTimeout(function(){throw n})}function sa(n){return n==="head"}function Ab(n,i){var r=i,u=0;do{var h=r.nextSibling;if(n.removeChild(r),h&&h.nodeType===8)if(r=h.data,r==="/$"||r==="/&"){if(u===0){n.removeChild(h),Qo(i);return}u--}else if(r==="$"||r==="$?"||r==="$~"||r==="$!"||r==="&")u++;else if(r==="html")gl(n.ownerDocument.documentElement);else if(r==="head"){r=n.ownerDocument.head,gl(r);for(var g=r.firstChild;g;){var x=g.nextSibling,M=g.nodeName;g[gn]||M==="SCRIPT"||M==="STYLE"||M==="LINK"&&g.rel.toLowerCase()==="stylesheet"||r.removeChild(g),g=x}}else r==="body"&&gl(n.ownerDocument.body);r=h}while(r);Qo(i)}function Ob(n,i){var r=n;n=0;do{var u=r.nextSibling;if(r.nodeType===1?i?(r._stashedDisplay=r.style.display,r.style.display="none"):(r.style.display=r._stashedDisplay||"",r.getAttribute("style")===""&&r.removeAttribute("style")):r.nodeType===3&&(i?(r._stashedText=r.nodeValue,r.nodeValue=""):r.nodeValue=r._stashedText||""),u&&u.nodeType===8)if(r=u.data,r==="/$"){if(n===0)break;n--}else r!=="$"&&r!=="$?"&&r!=="$~"&&r!=="$!"||n++;r=u}while(r)}function Gp(n){var i=n.firstChild;for(i&&i.nodeType===10&&(i=i.nextSibling);i;){var r=i;switch(i=i.nextSibling,r.nodeName){case"HTML":case"HEAD":case"BODY":Gp(r),yn(r);continue;case"SCRIPT":case"STYLE":continue;case"LINK":if(r.rel.toLowerCase()==="stylesheet")continue}n.removeChild(r)}}function aN(n,i,r,u){for(;n.nodeType===1;){var h=r;if(n.nodeName.toLowerCase()!==i.toLowerCase()){if(!u&&(n.nodeName!=="INPUT"||n.type!=="hidden"))break}else if(u){if(!n[gn])switch(i){case"meta":if(!n.hasAttribute("itemprop"))break;return n;case"link":if(g=n.getAttribute("rel"),g==="stylesheet"&&n.hasAttribute("data-precedence"))break;if(g!==h.rel||n.getAttribute("href")!==(h.href==null||h.href===""?null:h.href)||n.getAttribute("crossorigin")!==(h.crossOrigin==null?null:h.crossOrigin)||n.getAttribute("title")!==(h.title==null?null:h.title))break;return n;case"style":if(n.hasAttribute("data-precedence"))break;return n;case"script":if(g=n.getAttribute("src"),(g!==(h.src==null?null:h.src)||n.getAttribute("type")!==(h.type==null?null:h.type)||n.getAttribute("crossorigin")!==(h.crossOrigin==null?null:h.crossOrigin))&&g&&n.hasAttribute("async")&&!n.hasAttribute("itemprop"))break;return n;default:return n}}else if(i==="input"&&n.type==="hidden"){var g=h.name==null?null:""+h.name;if(h.type==="hidden"&&n.getAttribute("name")===g)return n}else return n;if(n=ms(n.nextSibling),n===null)break}return null}function oN(n,i,r){if(i==="")return null;for(;n.nodeType!==3;)if((n.nodeType!==1||n.nodeName!=="INPUT"||n.type!=="hidden")&&!r||(n=ms(n.nextSibling),n===null))return null;return n}function Mb(n,i){for(;n.nodeType!==8;)if((n.nodeType!==1||n.nodeName!=="INPUT"||n.type!=="hidden")&&!i||(n=ms(n.nextSibling),n===null))return null;return n}function Xp(n){return n.data==="$?"||n.data==="$~"}function Fp(n){return n.data==="$!"||n.data==="$?"&&n.ownerDocument.readyState!=="loading"}function rN(n,i){var r=n.ownerDocument;if(n.data==="$~")n._reactRetry=i;else if(n.data!=="$?"||r.readyState!=="loading")i();else{var u=function(){i(),r.removeEventListener("DOMContentLoaded",u)};r.addEventListener("DOMContentLoaded",u),n._reactRetry=u}}function ms(n){for(;n!=null;n=n.nextSibling){var i=n.nodeType;if(i===1||i===3)break;if(i===8){if(i=n.data,i==="$"||i==="$!"||i==="$?"||i==="$~"||i==="&"||i==="F!"||i==="F")break;if(i==="/$"||i==="/&")return null}}return n}var Jp=null;function zb(n){n=n.nextSibling;for(var i=0;n;){if(n.nodeType===8){var r=n.data;if(r==="/$"||r==="/&"){if(i===0)return ms(n.nextSibling);i--}else r!=="$"&&r!=="$!"&&r!=="$?"&&r!=="$~"&&r!=="&"||i++}n=n.nextSibling}return null}function Db(n){n=n.previousSibling;for(var i=0;n;){if(n.nodeType===8){var r=n.data;if(r==="$"||r==="$!"||r==="$?"||r==="$~"||r==="&"){if(i===0)return n;i--}else r!=="/$"&&r!=="/&"||i++}n=n.previousSibling}return null}function $b(n,i,r){switch(i=uu(r),n){case"html":if(n=i.documentElement,!n)throw Error(a(452));return n;case"head":if(n=i.head,!n)throw Error(a(453));return n;case"body":if(n=i.body,!n)throw Error(a(454));return n;default:throw Error(a(451))}}function gl(n){for(var i=n.attributes;i.length;)n.removeAttributeNode(i[0]);yn(n)}var hs=new Map,Rb=new Set;function du(n){return typeof n.getRootNode=="function"?n.getRootNode():n.nodeType===9?n:n.ownerDocument}var xi=U.d;U.d={f:lN,r:cN,D:uN,C:dN,L:fN,m:pN,X:hN,S:mN,M:gN};function lN(){var n=xi.f(),i=nu();return n||i}function cN(n){var i=Tn(n);i!==null&&i.tag===5&&i.type==="form"?Wy(i):xi.r(n)}var Xo=typeof document>"u"?null:document;function Lb(n,i,r){var u=Xo;if(u&&typeof i=="string"&&i){var h=Sn(i);h='link[rel="'+n+'"][href="'+h+'"]',typeof r=="string"&&(h+='[crossorigin="'+r+'"]'),Rb.has(h)||(Rb.add(h),n={rel:n,crossOrigin:r,href:i},u.querySelector(h)===null&&(i=u.createElement("link"),xn(i,"link",n),Dt(i),u.head.appendChild(i)))}}function uN(n){xi.D(n),Lb("dns-prefetch",n,null)}function dN(n,i){xi.C(n,i),Lb("preconnect",n,i)}function fN(n,i,r){xi.L(n,i,r);var u=Xo;if(u&&n&&i){var h='link[rel="preload"][as="'+Sn(i)+'"]';i==="image"&&r&&r.imageSrcSet?(h+='[imagesrcset="'+Sn(r.imageSrcSet)+'"]',typeof r.imageSizes=="string"&&(h+='[imagesizes="'+Sn(r.imageSizes)+'"]')):h+='[href="'+Sn(n)+'"]';var g=h;switch(i){case"style":g=Fo(n);break;case"script":g=Jo(n)}hs.has(g)||(n=y({rel:"preload",href:i==="image"&&r&&r.imageSrcSet?void 0:n,as:i},r),hs.set(g,n),u.querySelector(h)!==null||i==="style"&&u.querySelector(yl(g))||i==="script"&&u.querySelector(_l(g))||(i=u.createElement("link"),xn(i,"link",n),Dt(i),u.head.appendChild(i)))}}function pN(n,i){xi.m(n,i);var r=Xo;if(r&&n){var u=i&&typeof i.as=="string"?i.as:"script",h='link[rel="modulepreload"][as="'+Sn(u)+'"][href="'+Sn(n)+'"]',g=h;switch(u){case"audioworklet":case"paintworklet":case"serviceworker":case"sharedworker":case"worker":case"script":g=Jo(n)}if(!hs.has(g)&&(n=y({rel:"modulepreload",href:n},i),hs.set(g,n),r.querySelector(h)===null)){switch(u){case"audioworklet":case"paintworklet":case"serviceworker":case"sharedworker":case"worker":case"script":if(r.querySelector(_l(g)))return}u=r.createElement("link"),xn(u,"link",n),Dt(u),r.head.appendChild(u)}}}function mN(n,i,r){xi.S(n,i,r);var u=Xo;if(u&&n){var h=Cn(u).hoistableStyles,g=Fo(n);i=i||"default";var x=h.get(g);if(!x){var M={loading:0,preload:null};if(x=u.querySelector(yl(g)))M.loading=5;else{n=y({rel:"stylesheet",href:n,"data-precedence":i},r),(r=hs.get(g))&&Qp(n,r);var H=x=u.createElement("link");Dt(H),xn(H,"link",n),H._p=new Promise(function(se,fe){H.onload=se,H.onerror=fe}),H.addEventListener("load",function(){M.loading|=1}),H.addEventListener("error",function(){M.loading|=2}),M.loading|=4,fu(x,i,u)}x={type:"stylesheet",instance:x,count:1,state:M},h.set(g,x)}}}function hN(n,i){xi.X(n,i);var r=Xo;if(r&&n){var u=Cn(r).hoistableScripts,h=Jo(n),g=u.get(h);g||(g=r.querySelector(_l(h)),g||(n=y({src:n,async:!0},i),(i=hs.get(h))&&Wp(n,i),g=r.createElement("script"),Dt(g),xn(g,"link",n),r.head.appendChild(g)),g={type:"script",instance:g,count:1,state:null},u.set(h,g))}}function gN(n,i){xi.M(n,i);var r=Xo;if(r&&n){var u=Cn(r).hoistableScripts,h=Jo(n),g=u.get(h);g||(g=r.querySelector(_l(h)),g||(n=y({src:n,async:!0,type:"module"},i),(i=hs.get(h))&&Wp(n,i),g=r.createElement("script"),Dt(g),xn(g,"link",n),r.head.appendChild(g)),g={type:"script",instance:g,count:1,state:null},u.set(h,g))}}function qb(n,i,r,u){var h=(h=F.current)?du(h):null;if(!h)throw Error(a(446));switch(n){case"meta":case"title":return null;case"style":return typeof r.precedence=="string"&&typeof r.href=="string"?(i=Fo(r.href),r=Cn(h).hoistableStyles,u=r.get(i),u||(u={type:"style",instance:null,count:0,state:null},r.set(i,u)),u):{type:"void",instance:null,count:0,state:null};case"link":if(r.rel==="stylesheet"&&typeof r.href=="string"&&typeof r.precedence=="string"){n=Fo(r.href);var g=Cn(h).hoistableStyles,x=g.get(n);if(x||(h=h.ownerDocument||h,x={type:"stylesheet",instance:null,count:0,state:{loading:0,preload:null}},g.set(n,x),(g=h.querySelector(yl(n)))&&!g._p&&(x.instance=g,x.state.loading=5),hs.has(n)||(r={rel:"preload",as:"style",href:r.href,crossOrigin:r.crossOrigin,integrity:r.integrity,media:r.media,hrefLang:r.hrefLang,referrerPolicy:r.referrerPolicy},hs.set(n,r),g||yN(h,n,r,x.state))),i&&u===null)throw Error(a(528,""));return x}if(i&&u!==null)throw Error(a(529,""));return null;case"script":return i=r.async,r=r.src,typeof r=="string"&&i&&typeof i!="function"&&typeof i!="symbol"?(i=Jo(r),r=Cn(h).hoistableScripts,u=r.get(i),u||(u={type:"script",instance:null,count:0,state:null},r.set(i,u)),u):{type:"void",instance:null,count:0,state:null};default:throw Error(a(444,n))}}function Fo(n){return'href="'+Sn(n)+'"'}function yl(n){return'link[rel="stylesheet"]['+n+"]"}function Ub(n){return y({},n,{"data-precedence":n.precedence,precedence:null})}function yN(n,i,r,u){n.querySelector('link[rel="preload"][as="style"]['+i+"]")?u.loading=1:(i=n.createElement("link"),u.preload=i,i.addEventListener("load",function(){return u.loading|=1}),i.addEventListener("error",function(){return u.loading|=2}),xn(i,"link",r),Dt(i),n.head.appendChild(i))}function Jo(n){return'[src="'+Sn(n)+'"]'}function _l(n){return"script[async]"+n}function Bb(n,i,r){if(i.count++,i.instance===null)switch(i.type){case"style":var u=n.querySelector('style[data-href~="'+Sn(r.href)+'"]');if(u)return i.instance=u,Dt(u),u;var h=y({},r,{"data-href":r.href,"data-precedence":r.precedence,href:null,precedence:null});return u=(n.ownerDocument||n).createElement("style"),Dt(u),xn(u,"style",h),fu(u,r.precedence,n),i.instance=u;case"stylesheet":h=Fo(r.href);var g=n.querySelector(yl(h));if(g)return i.state.loading|=4,i.instance=g,Dt(g),g;u=Ub(r),(h=hs.get(h))&&Qp(u,h),g=(n.ownerDocument||n).createElement("link"),Dt(g);var x=g;return x._p=new Promise(function(M,H){x.onload=M,x.onerror=H}),xn(g,"link",u),i.state.loading|=4,fu(g,r.precedence,n),i.instance=g;case"script":return g=Jo(r.src),(h=n.querySelector(_l(g)))?(i.instance=h,Dt(h),h):(u=r,(h=hs.get(g))&&(u=y({},r),Wp(u,h)),n=n.ownerDocument||n,h=n.createElement("script"),Dt(h),xn(h,"link",u),n.head.appendChild(h),i.instance=h);case"void":return null;default:throw Error(a(443,i.type))}else i.type==="stylesheet"&&(i.state.loading&4)===0&&(u=i.instance,i.state.loading|=4,fu(u,r.precedence,n));return i.instance}function fu(n,i,r){for(var u=r.querySelectorAll('link[rel="stylesheet"][data-precedence],style[data-precedence]'),h=u.length?u[u.length-1]:null,g=h,x=0;x<u.length;x++){var M=u[x];if(M.dataset.precedence===i)g=M;else if(g!==h)break}g?g.parentNode.insertBefore(n,g.nextSibling):(i=r.nodeType===9?r.head:r,i.insertBefore(n,i.firstChild))}function Qp(n,i){n.crossOrigin==null&&(n.crossOrigin=i.crossOrigin),n.referrerPolicy==null&&(n.referrerPolicy=i.referrerPolicy),n.title==null&&(n.title=i.title)}function Wp(n,i){n.crossOrigin==null&&(n.crossOrigin=i.crossOrigin),n.referrerPolicy==null&&(n.referrerPolicy=i.referrerPolicy),n.integrity==null&&(n.integrity=i.integrity)}var pu=null;function Hb(n,i,r){if(pu===null){var u=new Map,h=pu=new Map;h.set(r,u)}else h=pu,u=h.get(r),u||(u=new Map,h.set(r,u));if(u.has(n))return u;for(u.set(n,null),r=r.getElementsByTagName(n),h=0;h<r.length;h++){var g=r[h];if(!(g[gn]||g[kt]||n==="link"&&g.getAttribute("rel")==="stylesheet")&&g.namespaceURI!=="http://www.w3.org/2000/svg"){var x=g.getAttribute(i)||"";x=n+x;var M=u.get(x);M?M.push(g):u.set(x,[g])}}return u}function Ib(n,i,r){n=n.ownerDocument||n,n.head.insertBefore(r,i==="title"?n.querySelector("head > title"):null)}function _N(n,i,r){if(r===1||i.itemProp!=null)return!1;switch(n){case"meta":case"title":return!0;case"style":if(typeof i.precedence!="string"||typeof i.href!="string"||i.href==="")break;return!0;case"link":if(typeof i.rel!="string"||typeof i.href!="string"||i.href===""||i.onLoad||i.onError)break;return i.rel==="stylesheet"?(n=i.disabled,typeof i.precedence=="string"&&n==null):!0;case"script":if(i.async&&typeof i.async!="function"&&typeof i.async!="symbol"&&!i.onLoad&&!i.onError&&i.src&&typeof i.src=="string")return!0}return!1}function Zb(n){return!(n.type==="stylesheet"&&(n.state.loading&3)===0)}function bN(n,i,r,u){if(r.type==="stylesheet"&&(typeof u.media!="string"||matchMedia(u.media).matches!==!1)&&(r.state.loading&4)===0){if(r.instance===null){var h=Fo(u.href),g=i.querySelector(yl(h));if(g){i=g._p,i!==null&&typeof i=="object"&&typeof i.then=="function"&&(n.count++,n=mu.bind(n),i.then(n,n)),r.state.loading|=4,r.instance=g,Dt(g);return}g=i.ownerDocument||i,u=Ub(u),(h=hs.get(h))&&Qp(u,h),g=g.createElement("link"),Dt(g);var x=g;x._p=new Promise(function(M,H){x.onload=M,x.onerror=H}),xn(g,"link",u),r.instance=g}n.stylesheets===null&&(n.stylesheets=new Map),n.stylesheets.set(r,i),(i=r.state.preload)&&(r.state.loading&3)===0&&(n.count++,r=mu.bind(n),i.addEventListener("load",r),i.addEventListener("error",r))}}var em=0;function vN(n,i){return n.stylesheets&&n.count===0&&gu(n,n.stylesheets),0<n.count||0<n.imgCount?function(r){var u=setTimeout(function(){if(n.stylesheets&&gu(n,n.stylesheets),n.unsuspend){var g=n.unsuspend;n.unsuspend=null,g()}},6e4+i);0<n.imgBytes&&em===0&&(em=62500*eN());var h=setTimeout(function(){if(n.waitingForImages=!1,n.count===0&&(n.stylesheets&&gu(n,n.stylesheets),n.unsuspend)){var g=n.unsuspend;n.unsuspend=null,g()}},(n.imgBytes>em?50:800)+i);return n.unsuspend=r,function(){n.unsuspend=null,clearTimeout(u),clearTimeout(h)}}:null}function mu(){if(this.count--,this.count===0&&(this.imgCount===0||!this.waitingForImages)){if(this.stylesheets)gu(this,this.stylesheets);else if(this.unsuspend){var n=this.unsuspend;this.unsuspend=null,n()}}}var hu=null;function gu(n,i){n.stylesheets=null,n.unsuspend!==null&&(n.count++,hu=new Map,i.forEach(xN,n),hu=null,mu.call(n))}function xN(n,i){if(!(i.state.loading&4)){var r=hu.get(n);if(r)var u=r.get(null);else{r=new Map,hu.set(n,r);for(var h=n.querySelectorAll("link[data-precedence],style[data-precedence]"),g=0;g<h.length;g++){var x=h[g];(x.nodeName==="LINK"||x.getAttribute("media")!=="not all")&&(r.set(x.dataset.precedence,x),u=x)}u&&r.set(null,u)}h=i.instance,x=h.getAttribute("data-precedence"),g=r.get(x)||u,g===u&&r.set(null,h),r.set(x,h),this.count++,u=mu.bind(this),h.addEventListener("load",u),h.addEventListener("error",u),g?g.parentNode.insertBefore(h,g.nextSibling):(n=n.nodeType===9?n.head:n,n.insertBefore(h,n.firstChild)),i.state.loading|=4}}var bl={$$typeof:N,Provider:null,Consumer:null,_currentValue:B,_currentValue2:B,_threadCount:0};function wN(n,i,r,u,h,g,x,M,H){this.tag=1,this.containerInfo=n,this.pingCache=this.current=this.pendingChildren=null,this.timeoutHandle=-1,this.callbackNode=this.next=this.pendingContext=this.context=this.cancelPendingCommit=null,this.callbackPriority=0,this.expirationTimes=vs(-1),this.entangledLanes=this.shellSuspendCounter=this.errorRecoveryDisabledLanes=this.expiredLanes=this.warmLanes=this.pingedLanes=this.suspendedLanes=this.pendingLanes=0,this.entanglements=vs(0),this.hiddenUpdates=vs(null),this.identifierPrefix=u,this.onUncaughtError=h,this.onCaughtError=g,this.onRecoverableError=x,this.pooledCache=null,this.pooledCacheLanes=0,this.formState=H,this.incompleteTransitions=new Map}function Pb(n,i,r,u,h,g,x,M,H,se,fe,ye){return n=new wN(n,i,r,x,H,se,fe,ye,M),i=1,g===!0&&(i|=24),g=Xn(3,null,null,i),n.current=g,g.stateNode=n,i=zf(),i.refCount++,n.pooledCache=i,i.refCount++,g.memoizedState={element:u,isDehydrated:r,cache:i},Lf(g),n}function Yb(n){return n?(n=To,n):To}function Vb(n,i,r,u,h,g){h=Yb(h),u.context===null?u.context=h:u.pendingContext=h,u=Vi(i),u.payload={element:r},g=g===void 0?null:g,g!==null&&(u.callback=g),r=Ki(n,u,i),r!==null&&(Hn(r,n,i),Jr(r,n,i))}function Kb(n,i){if(n=n.memoizedState,n!==null&&n.dehydrated!==null){var r=n.retryLane;n.retryLane=r!==0&&r<i?r:i}}function tm(n,i){Kb(n,i),(n=n.alternate)&&Kb(n,i)}function Gb(n){if(n.tag===13||n.tag===31){var i=$a(n,67108864);i!==null&&Hn(i,n,67108864),tm(n,67108864)}}function Xb(n){if(n.tag===13||n.tag===31){var i=es();i=Di(i);var r=$a(n,i);r!==null&&Hn(r,n,i),tm(n,i)}}var yu=!0;function SN(n,i,r,u){var h=D.T;D.T=null;var g=U.p;try{U.p=2,nm(n,i,r,u)}finally{U.p=g,D.T=h}}function kN(n,i,r,u){var h=D.T;D.T=null;var g=U.p;try{U.p=8,nm(n,i,r,u)}finally{U.p=g,D.T=h}}function nm(n,i,r,u){if(yu){var h=sm(u);if(h===null)Ip(n,i,u,_u,r),Jb(n,u);else if(NN(h,n,i,r,u))u.stopPropagation();else if(Jb(n,u),i&4&&-1<jN.indexOf(n)){for(;h!==null;){var g=Tn(h);if(g!==null)switch(g.tag){case 3:if(g=g.stateNode,g.current.memoizedState.isDehydrated){var x=Ce(g.pendingLanes);if(x!==0){var M=g;for(M.pendingLanes|=2,M.entangledLanes|=2;x;){var H=1<<31-Mt(x);M.entanglements[1]|=H,x&=~H}Vs(g),(ut&6)===0&&(eu=lt()+500,pl(0))}}break;case 31:case 13:M=$a(g,2),M!==null&&Hn(M,g,2),nu(),tm(g,2)}if(g=sm(u),g===null&&Ip(n,i,u,_u,r),g===h)break;h=g}h!==null&&u.stopPropagation()}else Ip(n,i,u,null,r)}}function sm(n){return n=dn(n),im(n)}var _u=null;function im(n){if(_u=null,n=un(n),n!==null){var i=l(n);if(i===null)n=null;else{var r=i.tag;if(r===13){if(n=c(i),n!==null)return n;n=null}else if(r===31){if(n=f(i),n!==null)return n;n=null}else if(r===3){if(i.stateNode.current.memoizedState.isDehydrated)return i.tag===3?i.stateNode.containerInfo:null;n=null}else i!==n&&(n=null)}}return _u=n,null}function Fb(n){switch(n){case"beforetoggle":case"cancel":case"click":case"close":case"contextmenu":case"copy":case"cut":case"auxclick":case"dblclick":case"dragend":case"dragstart":case"drop":case"focusin":case"focusout":case"input":case"invalid":case"keydown":case"keypress":case"keyup":case"mousedown":case"mouseup":case"paste":case"pause":case"play":case"pointercancel":case"pointerdown":case"pointerup":case"ratechange":case"reset":case"resize":case"seeked":case"submit":case"toggle":case"touchcancel":case"touchend":case"touchstart":case"volumechange":case"change":case"selectionchange":case"textInput":case"compositionstart":case"compositionend":case"compositionupdate":case"beforeblur":case"afterblur":case"beforeinput":case"blur":case"fullscreenchange":case"focus":case"hashchange":case"popstate":case"select":case"selectstart":return 2;case"drag":case"dragenter":case"dragexit":case"dragleave":case"dragover":case"mousemove":case"mouseout":case"mouseover":case"pointermove":case"pointerout":case"pointerover":case"scroll":case"touchmove":case"wheel":case"mouseenter":case"mouseleave":case"pointerenter":case"pointerleave":return 8;case"message":switch(St()){case mn:return 2;case Tt:return 8;case Ft:case Jt:return 32;case zn:return 268435456;default:return 32}default:return 32}}var am=!1,ia=null,aa=null,oa=null,vl=new Map,xl=new Map,ra=[],jN="mousedown mouseup touchcancel touchend touchstart auxclick dblclick pointercancel pointerdown pointerup dragend dragstart drop compositionend compositionstart keydown keypress keyup input textInput copy cut paste click change contextmenu reset".split(" ");function Jb(n,i){switch(n){case"focusin":case"focusout":ia=null;break;case"dragenter":case"dragleave":aa=null;break;case"mouseover":case"mouseout":oa=null;break;case"pointerover":case"pointerout":vl.delete(i.pointerId);break;case"gotpointercapture":case"lostpointercapture":xl.delete(i.pointerId)}}function wl(n,i,r,u,h,g){return n===null||n.nativeEvent!==g?(n={blockedOn:i,domEventName:r,eventSystemFlags:u,nativeEvent:g,targetContainers:[h]},i!==null&&(i=Tn(i),i!==null&&Gb(i)),n):(n.eventSystemFlags|=u,i=n.targetContainers,h!==null&&i.indexOf(h)===-1&&i.push(h),n)}function NN(n,i,r,u,h){switch(i){case"focusin":return ia=wl(ia,n,i,r,u,h),!0;case"dragenter":return aa=wl(aa,n,i,r,u,h),!0;case"mouseover":return oa=wl(oa,n,i,r,u,h),!0;case"pointerover":var g=h.pointerId;return vl.set(g,wl(vl.get(g)||null,n,i,r,u,h)),!0;case"gotpointercapture":return g=h.pointerId,xl.set(g,wl(xl.get(g)||null,n,i,r,u,h)),!0}return!1}function Qb(n){var i=un(n.target);if(i!==null){var r=l(i);if(r!==null){if(i=r.tag,i===13){if(i=c(r),i!==null){n.blockedOn=i,Sa(n.priority,function(){Xb(r)});return}}else if(i===31){if(i=f(r),i!==null){n.blockedOn=i,Sa(n.priority,function(){Xb(r)});return}}else if(i===3&&r.stateNode.current.memoizedState.isDehydrated){n.blockedOn=r.tag===3?r.stateNode.containerInfo:null;return}}}n.blockedOn=null}function bu(n){if(n.blockedOn!==null)return!1;for(var i=n.targetContainers;0<i.length;){var r=sm(n.nativeEvent);if(r===null){r=n.nativeEvent;var u=new r.constructor(r.type,r);Oe=u,r.target.dispatchEvent(u),Oe=null}else return i=Tn(r),i!==null&&Gb(i),n.blockedOn=r,!1;i.shift()}return!0}function Wb(n,i,r){bu(n)&&r.delete(i)}function EN(){am=!1,ia!==null&&bu(ia)&&(ia=null),aa!==null&&bu(aa)&&(aa=null),oa!==null&&bu(oa)&&(oa=null),vl.forEach(Wb),xl.forEach(Wb)}function vu(n,i){n.blockedOn===i&&(n.blockedOn=null,am||(am=!0,e.unstable_scheduleCallback(e.unstable_NormalPriority,EN)))}var xu=null;function ev(n){xu!==n&&(xu=n,e.unstable_scheduleCallback(e.unstable_NormalPriority,function(){xu===n&&(xu=null);for(var i=0;i<n.length;i+=3){var r=n[i],u=n[i+1],h=n[i+2];if(typeof u!="function"){if(im(u||r)===null)continue;break}var g=Tn(r);g!==null&&(n.splice(i,3),i-=3,sp(g,{pending:!0,data:h,method:r.method,action:u},u,h))}}))}function Qo(n){function i(H){return vu(H,n)}ia!==null&&vu(ia,n),aa!==null&&vu(aa,n),oa!==null&&vu(oa,n),vl.forEach(i),xl.forEach(i);for(var r=0;r<ra.length;r++){var u=ra[r];u.blockedOn===n&&(u.blockedOn=null)}for(;0<ra.length&&(r=ra[0],r.blockedOn===null);)Qb(r),r.blockedOn===null&&ra.shift();if(r=(n.ownerDocument||n).$$reactFormReplay,r!=null)for(u=0;u<r.length;u+=3){var h=r[u],g=r[u+1],x=h[Yt]||null;if(typeof g=="function")x||ev(r);else if(x){var M=null;if(g&&g.hasAttribute("formAction")){if(h=g,x=g[Yt]||null)M=x.formAction;else if(im(h)!==null)continue}else M=x.action;typeof M=="function"?r[u+1]=M:(r.splice(u,3),u-=3),ev(r)}}}function tv(){function n(g){g.canIntercept&&g.info==="react-transition"&&g.intercept({handler:function(){return new Promise(function(x){return h=x})},focusReset:"manual",scroll:"manual"})}function i(){h!==null&&(h(),h=null),u||setTimeout(r,20)}function r(){if(!u&&!navigation.transition){var g=navigation.currentEntry;g&&g.url!=null&&navigation.navigate(g.url,{state:g.getState(),info:"react-transition",history:"replace"})}}if(typeof navigation=="object"){var u=!1,h=null;return navigation.addEventListener("navigate",n),navigation.addEventListener("navigatesuccess",i),navigation.addEventListener("navigateerror",i),setTimeout(r,100),function(){u=!0,navigation.removeEventListener("navigate",n),navigation.removeEventListener("navigatesuccess",i),navigation.removeEventListener("navigateerror",i),h!==null&&(h(),h=null)}}}function om(n){this._internalRoot=n}wu.prototype.render=om.prototype.render=function(n){var i=this._internalRoot;if(i===null)throw Error(a(409));var r=i.current,u=es();Vb(r,u,n,i,null,null)},wu.prototype.unmount=om.prototype.unmount=function(){var n=this._internalRoot;if(n!==null){this._internalRoot=null;var i=n.containerInfo;Vb(n.current,2,null,n,null,null),nu(),i[as]=null}};function wu(n){this._internalRoot=n}wu.prototype.unstable_scheduleHydration=function(n){if(n){var i=wa();n={blockedOn:null,target:n,priority:i};for(var r=0;r<ra.length&&i!==0&&i<ra[r].priority;r++);ra.splice(r,0,n),r===0&&Qb(n)}};var nv=t.version;if(nv!=="19.2.6")throw Error(a(527,nv,"19.2.6"));U.findDOMNode=function(n){var i=n._reactInternals;if(i===void 0)throw typeof n.render=="function"?Error(a(188)):(n=Object.keys(n).join(","),Error(a(268,n)));return n=p(i),n=n!==null?_(n):null,n=n===null?null:n.stateNode,n};var TN={bundleType:0,version:"19.2.6",rendererPackageName:"react-dom",currentDispatcherRef:D,reconcilerVersion:"19.2.6"};if(typeof __REACT_DEVTOOLS_GLOBAL_HOOK__<"u"){var Su=__REACT_DEVTOOLS_GLOBAL_HOOK__;if(!Su.isDisabled&&Su.supportsFiber)try{Dn=Su.inject(TN),Qt=Su}catch{}}return kl.createRoot=function(n,i){if(!o(n))throw Error(a(299));var r=!1,u="",h=c_,g=u_,x=d_;return i!=null&&(i.unstable_strictMode===!0&&(r=!0),i.identifierPrefix!==void 0&&(u=i.identifierPrefix),i.onUncaughtError!==void 0&&(h=i.onUncaughtError),i.onCaughtError!==void 0&&(g=i.onCaughtError),i.onRecoverableError!==void 0&&(x=i.onRecoverableError)),i=Pb(n,1,!1,null,null,r,u,null,h,g,x,tv),n[as]=i.current,Hp(n),new om(i)},kl.hydrateRoot=function(n,i,r){if(!o(n))throw Error(a(299));var u=!1,h="",g=c_,x=u_,M=d_,H=null;return r!=null&&(r.unstable_strictMode===!0&&(u=!0),r.identifierPrefix!==void 0&&(h=r.identifierPrefix),r.onUncaughtError!==void 0&&(g=r.onUncaughtError),r.onCaughtError!==void 0&&(x=r.onCaughtError),r.onRecoverableError!==void 0&&(M=r.onRecoverableError),r.formState!==void 0&&(H=r.formState)),i=Pb(n,1,!0,i,r??null,u,h,H,g,x,M,tv),i.context=Yb(null),r=i.current,u=es(),u=Di(u),h=Vi(u),h.callback=null,Ki(r,h,u),r=u,i.current.lanes=r,Nn(i,r),Vs(i),n[as]=i.current,Hp(n),new wu(i)},kl.version="19.2.6",kl}var fv;function LN(){if(fv)return cm.exports;fv=1;function e(){if(!(typeof __REACT_DEVTOOLS_GLOBAL_HOOK__>"u"||typeof __REACT_DEVTOOLS_GLOBAL_HOOK__.checkDCE!="function"))try{__REACT_DEVTOOLS_GLOBAL_HOOK__.checkDCE(e)}catch(t){console.error(t)}}return e(),cm.exports=RN(),cm.exports}var qN=LN();class pv{constructor(t,s){this.op=t,this.value=s}op;value;test(t){const s=t.kind.toLowerCase(),a=this.value.toLowerCase();switch(this.op){case"=":return s===a;case"!=":return s!==a;case"~":return s.includes(a)}}}class UN{constructor(t,s){this.op=t,this.threshold=s}op;threshold;test(t){const s=BN(t);if(s===null)return!1;switch(this.op){case">":return s>this.threshold;case">=":return s>=this.threshold;case"<":return s<this.threshold;case"<=":return s<=this.threshold;case"=":return s===this.threshold;case"!=":return s!==this.threshold}}}function BN(e){const t=e.payload??{},s=["step_index","step","seq"];for(const a of s){const o=t[a];if(typeof o=="number"&&Number.isFinite(o))return o}return Number.isFinite(e.seq)?e.seq:null}function HN(e){const t=e.trim();if(t.length===0)return{predicate:()=>!0,error:null};const s=t.split(/\s+AND\s+/i),a=[];for(const l of s){const c=l.trim();if(c.length===0)return{predicate:null,error:"empty clause between AND"};const f=IN(c);if(f.error)return{predicate:null,error:f.error};a.push(f.clause)}return{predicate:l=>a.every(c=>c.test(l)),error:null}}function IN(e){const t=/^kind\s*~\s*(\S+)$/i.exec(e);if(t)return{clause:new pv("~",t[1])};const s=/^kind\s*(!=|=)\s*(\S+)$/i.exec(e);if(s)return{clause:new pv(s[1],s[2])};const a=/^step\s*(>=|<=|!=|>|<|=)\s*(-?\d+)$/i.exec(e);if(a){const o=a[1],l=Number.parseInt(a[2],10);return Number.isFinite(l)?{clause:new UN(o,l)}:{error:`clause '${e}': threshold is not a finite integer`}}return{error:`clause '${e}' did not match kind=…, kind!=…, kind~…, or step<op>N`}}function ZN(e,t){return t.predicate?e.filter(s=>t.predicate(s)):e}var mv;function ge(e,t,s){function a(f,m){if(f._zod||Object.defineProperty(f,"_zod",{value:{def:m,constr:c,traits:new Set},enumerable:!1}),f._zod.traits.has(e))return;f._zod.traits.add(e),t(f,m);const p=c.prototype,_=Object.keys(p);for(let y=0;y<_.length;y++){const b=_[y];b in f||(f[b]=p[b].bind(f))}}const o=s?.Parent??Object;class l extends o{}Object.defineProperty(l,"name",{value:e});function c(f){var m;const p=s?.Parent?new l:this;a(p,f),(m=p._zod).deferred??(m.deferred=[]);for(const _ of p._zod.deferred)_();return p}return Object.defineProperty(c,"init",{value:a}),Object.defineProperty(c,Symbol.hasInstance,{value:f=>s?.Parent&&f instanceof s.Parent?!0:f?._zod?.traits?.has(e)}),Object.defineProperty(c,"name",{value:e}),c}class ur extends Error{constructor(){super("Encountered Promise during synchronous parse. Use .parseAsync() instead.")}}class Bx extends Error{constructor(t){super(`Encountered unidirectional transform during encode: ${t}`),this.name="ZodEncodeError"}}(mv=globalThis).__zod_globalConfig??(mv.__zod_globalConfig={});const jh=globalThis.__zod_globalConfig;function Ai(e){return jh}function Hx(e){const t=Object.values(e).filter(a=>typeof a=="number");return Object.entries(e).filter(([a,o])=>t.indexOf(+a)===-1).map(([a,o])=>o)}function Km(e,t){return typeof t=="bigint"?t.toString():t}function Nh(e){return{get value(){{const t=e();return Object.defineProperty(this,"value",{value:t}),t}}}}function Eh(e){return e==null}function Th(e){const t=e.startsWith("^")?1:0,s=e.endsWith("$")?e.length-1:e.length;return e.slice(t,s)}function PN(e,t){const s=e/t,a=Math.round(s),o=Number.EPSILON*Math.max(Math.abs(s),1);return Math.abs(s-a)<o?0:s-a}const hv=Symbol("evaluating");function st(e,t,s){let a;Object.defineProperty(e,t,{get(){if(a!==hv)return a===void 0&&(a=hv,a=s()),a},set(o){Object.defineProperty(e,t,{value:o})},configurable:!0})}function ho(e,t,s){Object.defineProperty(e,t,{value:s,writable:!0,enumerable:!0,configurable:!0})}function ga(...e){const t={};for(const s of e){const a=Object.getOwnPropertyDescriptors(s);Object.assign(t,a)}return Object.defineProperties({},t)}function gv(e){return JSON.stringify(e)}function YN(e){return e.toLowerCase().trim().replace(/[^\w\s-]/g,"").replace(/[\s_-]+/g,"-").replace(/^-+|-+$/g,"")}const Ix="captureStackTrace"in Error?Error.captureStackTrace:(...e)=>{};function ed(e){return typeof e=="object"&&e!==null&&!Array.isArray(e)}const VN=Nh(()=>{if(jh.jitless||typeof navigator<"u"&&navigator?.userAgent?.includes("Cloudflare"))return!1;try{const e=Function;return new e(""),!0}catch{return!1}});function _r(e){if(ed(e)===!1)return!1;const t=e.constructor;if(t===void 0||typeof t!="function")return!0;const s=t.prototype;return!(ed(s)===!1||Object.prototype.hasOwnProperty.call(s,"isPrototypeOf")===!1)}function Zx(e){return _r(e)?{...e}:Array.isArray(e)?[...e]:e instanceof Map?new Map(e):e instanceof Set?new Set(e):e}const KN=new Set(["string","number","symbol"]);function Ed(e){return e.replace(/[.*+?^${}()|[\]\\]/g,"\\$&")}function ya(e,t,s){const a=new e._zod.constr(t??e._zod.def);return(!t||s?.parent)&&(a._zod.parent=e),a}function De(e){const t=e;if(!t)return{};if(typeof t=="string")return{error:()=>t};if(t?.message!==void 0){if(t?.error!==void 0)throw new Error("Cannot specify both `message` and `error` params");t.error=t.message}return delete t.message,typeof t.error=="string"?{...t,error:()=>t.error}:t}function GN(e){return Object.keys(e).filter(t=>e[t]._zod.optin==="optional"&&e[t]._zod.optout==="optional")}const XN={safeint:[Number.MIN_SAFE_INTEGER,Number.MAX_SAFE_INTEGER],int32:[-2147483648,2147483647],uint32:[0,4294967295],float32:[-34028234663852886e22,34028234663852886e22],float64:[-Number.MAX_VALUE,Number.MAX_VALUE]};function FN(e,t){const s=e._zod.def,a=s.checks;if(a&&a.length>0)throw new Error(".pick() cannot be used on object schemas containing refinements");const l=ga(e._zod.def,{get shape(){const c={};for(const f in t){if(!(f in s.shape))throw new Error(`Unrecognized key: "${f}"`);t[f]&&(c[f]=s.shape[f])}return ho(this,"shape",c),c},checks:[]});return ya(e,l)}function JN(e,t){const s=e._zod.def,a=s.checks;if(a&&a.length>0)throw new Error(".omit() cannot be used on object schemas containing refinements");const l=ga(e._zod.def,{get shape(){const c={...e._zod.def.shape};for(const f in t){if(!(f in s.shape))throw new Error(`Unrecognized key: "${f}"`);t[f]&&delete c[f]}return ho(this,"shape",c),c},checks:[]});return ya(e,l)}function QN(e,t){if(!_r(t))throw new Error("Invalid input to extend: expected a plain object");const s=e._zod.def.checks;if(s&&s.length>0){const l=e._zod.def.shape;for(const c in t)if(Object.getOwnPropertyDescriptor(l,c)!==void 0)throw new Error("Cannot overwrite keys on object schemas containing refinements. Use `.safeExtend()` instead.")}const o=ga(e._zod.def,{get shape(){const l={...e._zod.def.shape,...t};return ho(this,"shape",l),l}});return ya(e,o)}function WN(e,t){if(!_r(t))throw new Error("Invalid input to safeExtend: expected a plain object");const s=ga(e._zod.def,{get shape(){const a={...e._zod.def.shape,...t};return ho(this,"shape",a),a}});return ya(e,s)}function eE(e,t){if(e._zod.def.checks?.length)throw new Error(".merge() cannot be used on object schemas containing refinements. Use .safeExtend() instead.");const s=ga(e._zod.def,{get shape(){const a={...e._zod.def.shape,...t._zod.def.shape};return ho(this,"shape",a),a},get catchall(){return t._zod.def.catchall},checks:t._zod.def.checks??[]});return ya(e,s)}function tE(e,t,s){const o=t._zod.def.checks;if(o&&o.length>0)throw new Error(".partial() cannot be used on object schemas containing refinements");const c=ga(t._zod.def,{get shape(){const f=t._zod.def.shape,m={...f};if(s)for(const p in s){if(!(p in f))throw new Error(`Unrecognized key: "${p}"`);s[p]&&(m[p]=e?new e({type:"optional",innerType:f[p]}):f[p])}else for(const p in f)m[p]=e?new e({type:"optional",innerType:f[p]}):f[p];return ho(this,"shape",m),m},checks:[]});return ya(t,c)}function nE(e,t,s){const a=ga(t._zod.def,{get shape(){const o=t._zod.def.shape,l={...o};if(s)for(const c in s){if(!(c in l))throw new Error(`Unrecognized key: "${c}"`);s[c]&&(l[c]=new e({type:"nonoptional",innerType:o[c]}))}else for(const c in o)l[c]=new e({type:"nonoptional",innerType:o[c]});return ho(this,"shape",l),l}});return ya(t,a)}function sr(e,t=0){if(e.aborted===!0)return!0;for(let s=t;s<e.issues.length;s++)if(e.issues[s]?.continue!==!0)return!0;return!1}function sE(e,t=0){if(e.aborted===!0)return!0;for(let s=t;s<e.issues.length;s++)if(e.issues[s]?.continue===!1)return!0;return!1}function ir(e,t){return t.map(s=>{var a;return(a=s).path??(a.path=[]),s.path.unshift(e),s})}function ku(e){return typeof e=="string"?e:e?.message}function Oi(e,t,s){const a=e.message?e.message:ku(e.inst?._zod.def?.error?.(e))??ku(t?.error?.(e))??ku(s.customError?.(e))??ku(s.localeError?.(e))??"Invalid input",{inst:o,continue:l,input:c,...f}=e;return f.path??(f.path=[]),f.message=a,t?.reportInput&&(f.input=c),f}function Ch(e){return Array.isArray(e)?"array":typeof e=="string"?"string":"unknown"}function Ll(...e){const[t,s,a]=e;return typeof t=="string"?{message:t,code:"custom",input:s,inst:a}:{...t}}const Px=(e,t)=>{e.name="$ZodError",Object.defineProperty(e,"_zod",{value:e._zod,enumerable:!1}),Object.defineProperty(e,"issues",{value:t,enumerable:!1}),e.message=JSON.stringify(t,Km,2),Object.defineProperty(e,"toString",{value:()=>e.message,enumerable:!1})},Yx=ge("$ZodError",Px),Vx=ge("$ZodError",Px,{Parent:Error});function iE(e,t=s=>s.message){const s={},a=[];for(const o of e.issues)o.path.length>0?(s[o.path[0]]=s[o.path[0]]||[],s[o.path[0]].push(t(o))):a.push(t(o));return{formErrors:a,fieldErrors:s}}function aE(e,t=s=>s.message){const s={_errors:[]},a=(o,l=[])=>{for(const c of o.issues)if(c.code==="invalid_union"&&c.errors.length)c.errors.map(f=>a({issues:f},[...l,...c.path]));else if(c.code==="invalid_key")a({issues:c.issues},[...l,...c.path]);else if(c.code==="invalid_element")a({issues:c.issues},[...l,...c.path]);else{const f=[...l,...c.path];if(f.length===0)s._errors.push(t(c));else{let m=s,p=0;for(;p<f.length;){const _=f[p];p===f.length-1?(m[_]=m[_]||{_errors:[]},m[_]._errors.push(t(c))):m[_]=m[_]||{_errors:[]},m=m[_],p++}}}};return a(e),s}const Ah=e=>(t,s,a,o)=>{const l=a?{...a,async:!1}:{async:!1},c=t._zod.run({value:s,issues:[]},l);if(c instanceof Promise)throw new ur;if(c.issues.length){const f=new(o?.Err??e)(c.issues.map(m=>Oi(m,l,Ai())));throw Ix(f,o?.callee),f}return c.value},Oh=e=>async(t,s,a,o)=>{const l=a?{...a,async:!0}:{async:!0};let c=t._zod.run({value:s,issues:[]},l);if(c instanceof Promise&&(c=await c),c.issues.length){const f=new(o?.Err??e)(c.issues.map(m=>Oi(m,l,Ai())));throw Ix(f,o?.callee),f}return c.value},Td=e=>(t,s,a)=>{const o=a?{...a,async:!1}:{async:!1},l=t._zod.run({value:s,issues:[]},o);if(l instanceof Promise)throw new ur;return l.issues.length?{success:!1,error:new(e??Yx)(l.issues.map(c=>Oi(c,o,Ai())))}:{success:!0,data:l.value}},oE=Td(Vx),Cd=e=>async(t,s,a)=>{const o=a?{...a,async:!0}:{async:!0};let l=t._zod.run({value:s,issues:[]},o);return l instanceof Promise&&(l=await l),l.issues.length?{success:!1,error:new e(l.issues.map(c=>Oi(c,o,Ai())))}:{success:!0,data:l.value}},rE=Cd(Vx),lE=e=>(t,s,a)=>{const o=a?{...a,direction:"backward"}:{direction:"backward"};return Ah(e)(t,s,o)},cE=e=>(t,s,a)=>Ah(e)(t,s,a),uE=e=>async(t,s,a)=>{const o=a?{...a,direction:"backward"}:{direction:"backward"};return Oh(e)(t,s,o)},dE=e=>async(t,s,a)=>Oh(e)(t,s,a),fE=e=>(t,s,a)=>{const o=a?{...a,direction:"backward"}:{direction:"backward"};return Td(e)(t,s,o)},pE=e=>(t,s,a)=>Td(e)(t,s,a),mE=e=>async(t,s,a)=>{const o=a?{...a,direction:"backward"}:{direction:"backward"};return Cd(e)(t,s,o)},hE=e=>async(t,s,a)=>Cd(e)(t,s,a),gE=/^[cC][0-9a-z]{6,}$/,yE=/^[0-9a-z]+$/,_E=/^[0-9A-HJKMNP-TV-Za-hjkmnp-tv-z]{26}$/,bE=/^[0-9a-vA-V]{20}$/,vE=/^[A-Za-z0-9]{27}$/,xE=/^[a-zA-Z0-9_-]{21}$/,wE=/^P(?:(\d+W)|(?!.*W)(?=\d|T\d)(\d+Y)?(\d+M)?(\d+D)?(T(?=\d)(\d+H)?(\d+M)?(\d+([.,]\d+)?S)?)?)$/,SE=/^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})$/,yv=e=>e?new RegExp(`^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-${e}[0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12})$`):/^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000|ffffffff-ffff-ffff-ffff-ffffffffffff)$/,kE=/^(?!\.)(?!.*\.\.)([A-Za-z0-9_'+\-\.]*)[A-Za-z0-9_+-]@([A-Za-z0-9][A-Za-z0-9\-]*\.)+[A-Za-z]{2,}$/,jE="^(\\p{Extended_Pictographic}|\\p{Emoji_Component})+$";function NE(){return new RegExp(jE,"u")}const EE=/^(?:(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])\.){3}(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])$/,TE=/^(([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,7}:|([0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,5}(:[0-9a-fA-F]{1,4}){1,2}|([0-9a-fA-F]{1,4}:){1,4}(:[0-9a-fA-F]{1,4}){1,3}|([0-9a-fA-F]{1,4}:){1,3}(:[0-9a-fA-F]{1,4}){1,4}|([0-9a-fA-F]{1,4}:){1,2}(:[0-9a-fA-F]{1,4}){1,5}|[0-9a-fA-F]{1,4}:((:[0-9a-fA-F]{1,4}){1,6})|:((:[0-9a-fA-F]{1,4}){1,7}|:))$/,CE=/^((25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])\.){3}(25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])\/([0-9]|[1-2][0-9]|3[0-2])$/,AE=/^(([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}|::|([0-9a-fA-F]{1,4})?::([0-9a-fA-F]{1,4}:?){0,6})\/(12[0-8]|1[01][0-9]|[1-9]?[0-9])$/,OE=/^$|^(?:[0-9a-zA-Z+/]{4})*(?:(?:[0-9a-zA-Z+/]{2}==)|(?:[0-9a-zA-Z+/]{3}=))?$/,Kx=/^[A-Za-z0-9_-]*$/,ME=/^https?$/,zE=/^\+[1-9]\d{6,14}$/,Gx="(?:(?:\\d\\d[2468][048]|\\d\\d[13579][26]|\\d\\d0[48]|[02468][048]00|[13579][26]00)-02-29|\\d{4}-(?:(?:0[13578]|1[02])-(?:0[1-9]|[12]\\d|3[01])|(?:0[469]|11)-(?:0[1-9]|[12]\\d|30)|(?:02)-(?:0[1-9]|1\\d|2[0-8])))",DE=new RegExp(`^${Gx}$`);function Xx(e){const t="(?:[01]\\d|2[0-3]):[0-5]\\d";return typeof e.precision=="number"?e.precision===-1?`${t}`:e.precision===0?`${t}:[0-5]\\d`:`${t}:[0-5]\\d\\.\\d{${e.precision}}`:`${t}(?::[0-5]\\d(?:\\.\\d+)?)?`}function $E(e){return new RegExp(`^${Xx(e)}$`)}function RE(e){const t=Xx({precision:e.precision}),s=["Z"];e.local&&s.push(""),e.offset&&s.push("([+-](?:[01]\\d|2[0-3]):[0-5]\\d)");const a=`${t}(?:${s.join("|")})`;return new RegExp(`^${Gx}T(?:${a})$`)}const LE=e=>{const t=e?`[\\s\\S]{${e?.minimum??0},${e?.maximum??""}}`:"[\\s\\S]*";return new RegExp(`^${t}$`)},qE=/^-?\d+$/,Fx=/^-?\d+(?:\.\d+)?$/,UE=/^(?:true|false)$/i,BE=/^[^A-Z]*$/,HE=/^[^a-z]*$/,Yn=ge("$ZodCheck",(e,t)=>{var s;e._zod??(e._zod={}),e._zod.def=t,(s=e._zod).onattach??(s.onattach=[])}),Jx={number:"number",bigint:"bigint",object:"date"},Qx=ge("$ZodCheckLessThan",(e,t)=>{Yn.init(e,t);const s=Jx[typeof t.value];e._zod.onattach.push(a=>{const o=a._zod.bag,l=(t.inclusive?o.maximum:o.exclusiveMaximum)??Number.POSITIVE_INFINITY;t.value<l&&(t.inclusive?o.maximum=t.value:o.exclusiveMaximum=t.value)}),e._zod.check=a=>{(t.inclusive?a.value<=t.value:a.value<t.value)||a.issues.push({origin:s,code:"too_big",maximum:typeof t.value=="object"?t.value.getTime():t.value,input:a.value,inclusive:t.inclusive,inst:e,continue:!t.abort})}}),Wx=ge("$ZodCheckGreaterThan",(e,t)=>{Yn.init(e,t);const s=Jx[typeof t.value];e._zod.onattach.push(a=>{const o=a._zod.bag,l=(t.inclusive?o.minimum:o.exclusiveMinimum)??Number.NEGATIVE_INFINITY;t.value>l&&(t.inclusive?o.minimum=t.value:o.exclusiveMinimum=t.value)}),e._zod.check=a=>{(t.inclusive?a.value>=t.value:a.value>t.value)||a.issues.push({origin:s,code:"too_small",minimum:typeof t.value=="object"?t.value.getTime():t.value,input:a.value,inclusive:t.inclusive,inst:e,continue:!t.abort})}}),IE=ge("$ZodCheckMultipleOf",(e,t)=>{Yn.init(e,t),e._zod.onattach.push(s=>{var a;(a=s._zod.bag).multipleOf??(a.multipleOf=t.value)}),e._zod.check=s=>{if(typeof s.value!=typeof t.value)throw new Error("Cannot mix number and bigint in multiple_of check.");(typeof s.value=="bigint"?s.value%t.value===BigInt(0):PN(s.value,t.value)===0)||s.issues.push({origin:typeof s.value,code:"not_multiple_of",divisor:t.value,input:s.value,inst:e,continue:!t.abort})}}),ZE=ge("$ZodCheckNumberFormat",(e,t)=>{Yn.init(e,t),t.format=t.format||"float64";const s=t.format?.includes("int"),a=s?"int":"number",[o,l]=XN[t.format];e._zod.onattach.push(c=>{const f=c._zod.bag;f.format=t.format,f.minimum=o,f.maximum=l,s&&(f.pattern=qE)}),e._zod.check=c=>{const f=c.value;if(s){if(!Number.isInteger(f)){c.issues.push({expected:a,format:t.format,code:"invalid_type",continue:!1,input:f,inst:e});return}if(!Number.isSafeInteger(f)){f>0?c.issues.push({input:f,code:"too_big",maximum:Number.MAX_SAFE_INTEGER,note:"Integers must be within the safe integer range.",inst:e,origin:a,inclusive:!0,continue:!t.abort}):c.issues.push({input:f,code:"too_small",minimum:Number.MIN_SAFE_INTEGER,note:"Integers must be within the safe integer range.",inst:e,origin:a,inclusive:!0,continue:!t.abort});return}}f<o&&c.issues.push({origin:"number",input:f,code:"too_small",minimum:o,inclusive:!0,inst:e,continue:!t.abort}),f>l&&c.issues.push({origin:"number",input:f,code:"too_big",maximum:l,inclusive:!0,inst:e,continue:!t.abort})}}),PE=ge("$ZodCheckMaxLength",(e,t)=>{var s;Yn.init(e,t),(s=e._zod.def).when??(s.when=a=>{const o=a.value;return!Eh(o)&&o.length!==void 0}),e._zod.onattach.push(a=>{const o=a._zod.bag.maximum??Number.POSITIVE_INFINITY;t.maximum<o&&(a._zod.bag.maximum=t.maximum)}),e._zod.check=a=>{const o=a.value;if(o.length<=t.maximum)return;const c=Ch(o);a.issues.push({origin:c,code:"too_big",maximum:t.maximum,inclusive:!0,input:o,inst:e,continue:!t.abort})}}),YE=ge("$ZodCheckMinLength",(e,t)=>{var s;Yn.init(e,t),(s=e._zod.def).when??(s.when=a=>{const o=a.value;return!Eh(o)&&o.length!==void 0}),e._zod.onattach.push(a=>{const o=a._zod.bag.minimum??Number.NEGATIVE_INFINITY;t.minimum>o&&(a._zod.bag.minimum=t.minimum)}),e._zod.check=a=>{const o=a.value;if(o.length>=t.minimum)return;const c=Ch(o);a.issues.push({origin:c,code:"too_small",minimum:t.minimum,inclusive:!0,input:o,inst:e,continue:!t.abort})}}),VE=ge("$ZodCheckLengthEquals",(e,t)=>{var s;Yn.init(e,t),(s=e._zod.def).when??(s.when=a=>{const o=a.value;return!Eh(o)&&o.length!==void 0}),e._zod.onattach.push(a=>{const o=a._zod.bag;o.minimum=t.length,o.maximum=t.length,o.length=t.length}),e._zod.check=a=>{const o=a.value,l=o.length;if(l===t.length)return;const c=Ch(o),f=l>t.length;a.issues.push({origin:c,...f?{code:"too_big",maximum:t.length}:{code:"too_small",minimum:t.length},inclusive:!0,exact:!0,input:a.value,inst:e,continue:!t.abort})}}),Ad=ge("$ZodCheckStringFormat",(e,t)=>{var s,a;Yn.init(e,t),e._zod.onattach.push(o=>{const l=o._zod.bag;l.format=t.format,t.pattern&&(l.patterns??(l.patterns=new Set),l.patterns.add(t.pattern))}),t.pattern?(s=e._zod).check??(s.check=o=>{t.pattern.lastIndex=0,!t.pattern.test(o.value)&&o.issues.push({origin:"string",code:"invalid_format",format:t.format,input:o.value,...t.pattern?{pattern:t.pattern.toString()}:{},inst:e,continue:!t.abort})}):(a=e._zod).check??(a.check=()=>{})}),KE=ge("$ZodCheckRegex",(e,t)=>{Ad.init(e,t),e._zod.check=s=>{t.pattern.lastIndex=0,!t.pattern.test(s.value)&&s.issues.push({origin:"string",code:"invalid_format",format:"regex",input:s.value,pattern:t.pattern.toString(),inst:e,continue:!t.abort})}}),GE=ge("$ZodCheckLowerCase",(e,t)=>{t.pattern??(t.pattern=BE),Ad.init(e,t)}),XE=ge("$ZodCheckUpperCase",(e,t)=>{t.pattern??(t.pattern=HE),Ad.init(e,t)}),FE=ge("$ZodCheckIncludes",(e,t)=>{Yn.init(e,t);const s=Ed(t.includes),a=new RegExp(typeof t.position=="number"?`^.{${t.position}}${s}`:s);t.pattern=a,e._zod.onattach.push(o=>{const l=o._zod.bag;l.patterns??(l.patterns=new Set),l.patterns.add(a)}),e._zod.check=o=>{o.value.includes(t.includes,t.position)||o.issues.push({origin:"string",code:"invalid_format",format:"includes",includes:t.includes,input:o.value,inst:e,continue:!t.abort})}}),JE=ge("$ZodCheckStartsWith",(e,t)=>{Yn.init(e,t);const s=new RegExp(`^${Ed(t.prefix)}.*`);t.pattern??(t.pattern=s),e._zod.onattach.push(a=>{const o=a._zod.bag;o.patterns??(o.patterns=new Set),o.patterns.add(s)}),e._zod.check=a=>{a.value.startsWith(t.prefix)||a.issues.push({origin:"string",code:"invalid_format",format:"starts_with",prefix:t.prefix,input:a.value,inst:e,continue:!t.abort})}}),QE=ge("$ZodCheckEndsWith",(e,t)=>{Yn.init(e,t);const s=new RegExp(`.*${Ed(t.suffix)}$`);t.pattern??(t.pattern=s),e._zod.onattach.push(a=>{const o=a._zod.bag;o.patterns??(o.patterns=new Set),o.patterns.add(s)}),e._zod.check=a=>{a.value.endsWith(t.suffix)||a.issues.push({origin:"string",code:"invalid_format",format:"ends_with",suffix:t.suffix,input:a.value,inst:e,continue:!t.abort})}}),WE=ge("$ZodCheckOverwrite",(e,t)=>{Yn.init(e,t),e._zod.check=s=>{s.value=t.tx(s.value)}});class e2{constructor(t=[]){this.content=[],this.indent=0,this&&(this.args=t)}indented(t){this.indent+=1,t(this),this.indent-=1}write(t){if(typeof t=="function"){t(this,{execution:"sync"}),t(this,{execution:"async"});return}const a=t.split(`
`).filter(c=>c),o=Math.min(...a.map(c=>c.length-c.trimStart().length)),l=a.map(c=>c.slice(o)).map(c=>" ".repeat(this.indent*2)+c);for(const c of l)this.content.push(c)}compile(){const t=Function,s=this?.args,o=[...(this?.content??[""]).map(l=>`  ${l}`)];return new t(...s,o.join(`
`))}}const t2={major:4,minor:4,patch:3},qt=ge("$ZodType",(e,t)=>{var s;e??(e={}),e._zod.def=t,e._zod.bag=e._zod.bag||{},e._zod.version=t2;const a=[...e._zod.def.checks??[]];e._zod.traits.has("$ZodCheck")&&a.unshift(e);for(const o of a)for(const l of o._zod.onattach)l(e);if(a.length===0)(s=e._zod).deferred??(s.deferred=[]),e._zod.deferred?.push(()=>{e._zod.run=e._zod.parse});else{const o=(c,f,m)=>{let p=sr(c),_;for(const y of f){if(y._zod.def.when){if(sE(c)||!y._zod.def.when(c))continue}else if(p)continue;const b=c.issues.length,v=y._zod.check(c);if(v instanceof Promise&&m?.async===!1)throw new ur;if(_||v instanceof Promise)_=(_??Promise.resolve()).then(async()=>{await v,c.issues.length!==b&&(p||(p=sr(c,b)))});else{if(c.issues.length===b)continue;p||(p=sr(c,b))}}return _?_.then(()=>c):c},l=(c,f,m)=>{if(sr(c))return c.aborted=!0,c;const p=o(f,a,m);if(p instanceof Promise){if(m.async===!1)throw new ur;return p.then(_=>e._zod.parse(_,m))}return e._zod.parse(p,m)};e._zod.run=(c,f)=>{if(f.skipChecks)return e._zod.parse(c,f);if(f.direction==="backward"){const p=e._zod.parse({value:c.value,issues:[]},{...f,skipChecks:!0});return p instanceof Promise?p.then(_=>l(_,c,f)):l(p,c,f)}const m=e._zod.parse(c,f);if(m instanceof Promise){if(f.async===!1)throw new ur;return m.then(p=>o(p,a,f))}return o(m,a,f)}}st(e,"~standard",()=>({validate:o=>{try{const l=oE(e,o);return l.success?{value:l.data}:{issues:l.error?.issues}}catch{return rE(e,o).then(c=>c.success?{value:c.data}:{issues:c.error?.issues})}},vendor:"zod",version:1}))}),Mh=ge("$ZodString",(e,t)=>{qt.init(e,t),e._zod.pattern=[...e?._zod.bag?.patterns??[]].pop()??LE(e._zod.bag),e._zod.parse=(s,a)=>{if(t.coerce)try{s.value=String(s.value)}catch{}return typeof s.value=="string"||s.issues.push({expected:"string",code:"invalid_type",input:s.value,inst:e}),s}}),Ot=ge("$ZodStringFormat",(e,t)=>{Ad.init(e,t),Mh.init(e,t)}),n2=ge("$ZodGUID",(e,t)=>{t.pattern??(t.pattern=SE),Ot.init(e,t)}),s2=ge("$ZodUUID",(e,t)=>{if(t.version){const a={v1:1,v2:2,v3:3,v4:4,v5:5,v6:6,v7:7,v8:8}[t.version];if(a===void 0)throw new Error(`Invalid UUID version: "${t.version}"`);t.pattern??(t.pattern=yv(a))}else t.pattern??(t.pattern=yv());Ot.init(e,t)}),i2=ge("$ZodEmail",(e,t)=>{t.pattern??(t.pattern=kE),Ot.init(e,t)}),a2=ge("$ZodURL",(e,t)=>{Ot.init(e,t),e._zod.check=s=>{try{const a=s.value.trim();if(!t.normalize&&t.protocol?.source===ME.source&&!/^https?:\/\//i.test(a)){s.issues.push({code:"invalid_format",format:"url",note:"Invalid URL format",input:s.value,inst:e,continue:!t.abort});return}const o=new URL(a);t.hostname&&(t.hostname.lastIndex=0,t.hostname.test(o.hostname)||s.issues.push({code:"invalid_format",format:"url",note:"Invalid hostname",pattern:t.hostname.source,input:s.value,inst:e,continue:!t.abort})),t.protocol&&(t.protocol.lastIndex=0,t.protocol.test(o.protocol.endsWith(":")?o.protocol.slice(0,-1):o.protocol)||s.issues.push({code:"invalid_format",format:"url",note:"Invalid protocol",pattern:t.protocol.source,input:s.value,inst:e,continue:!t.abort})),t.normalize?s.value=o.href:s.value=a;return}catch{s.issues.push({code:"invalid_format",format:"url",input:s.value,inst:e,continue:!t.abort})}}}),o2=ge("$ZodEmoji",(e,t)=>{t.pattern??(t.pattern=NE()),Ot.init(e,t)}),r2=ge("$ZodNanoID",(e,t)=>{t.pattern??(t.pattern=xE),Ot.init(e,t)}),l2=ge("$ZodCUID",(e,t)=>{t.pattern??(t.pattern=gE),Ot.init(e,t)}),c2=ge("$ZodCUID2",(e,t)=>{t.pattern??(t.pattern=yE),Ot.init(e,t)}),u2=ge("$ZodULID",(e,t)=>{t.pattern??(t.pattern=_E),Ot.init(e,t)}),d2=ge("$ZodXID",(e,t)=>{t.pattern??(t.pattern=bE),Ot.init(e,t)}),f2=ge("$ZodKSUID",(e,t)=>{t.pattern??(t.pattern=vE),Ot.init(e,t)}),p2=ge("$ZodISODateTime",(e,t)=>{t.pattern??(t.pattern=RE(t)),Ot.init(e,t)}),m2=ge("$ZodISODate",(e,t)=>{t.pattern??(t.pattern=DE),Ot.init(e,t)}),h2=ge("$ZodISOTime",(e,t)=>{t.pattern??(t.pattern=$E(t)),Ot.init(e,t)}),g2=ge("$ZodISODuration",(e,t)=>{t.pattern??(t.pattern=wE),Ot.init(e,t)}),y2=ge("$ZodIPv4",(e,t)=>{t.pattern??(t.pattern=EE),Ot.init(e,t),e._zod.bag.format="ipv4"}),_2=ge("$ZodIPv6",(e,t)=>{t.pattern??(t.pattern=TE),Ot.init(e,t),e._zod.bag.format="ipv6",e._zod.check=s=>{try{new URL(`http://[${s.value}]`)}catch{s.issues.push({code:"invalid_format",format:"ipv6",input:s.value,inst:e,continue:!t.abort})}}}),b2=ge("$ZodCIDRv4",(e,t)=>{t.pattern??(t.pattern=CE),Ot.init(e,t)}),v2=ge("$ZodCIDRv6",(e,t)=>{t.pattern??(t.pattern=AE),Ot.init(e,t),e._zod.check=s=>{const a=s.value.split("/");try{if(a.length!==2)throw new Error;const[o,l]=a;if(!l)throw new Error;const c=Number(l);if(`${c}`!==l)throw new Error;if(c<0||c>128)throw new Error;new URL(`http://[${o}]`)}catch{s.issues.push({code:"invalid_format",format:"cidrv6",input:s.value,inst:e,continue:!t.abort})}}});function ew(e){if(e==="")return!0;if(/\s/.test(e)||e.length%4!==0)return!1;try{return atob(e),!0}catch{return!1}}const x2=ge("$ZodBase64",(e,t)=>{t.pattern??(t.pattern=OE),Ot.init(e,t),e._zod.bag.contentEncoding="base64",e._zod.check=s=>{ew(s.value)||s.issues.push({code:"invalid_format",format:"base64",input:s.value,inst:e,continue:!t.abort})}});function w2(e){if(!Kx.test(e))return!1;const t=e.replace(/[-_]/g,a=>a==="-"?"+":"/"),s=t.padEnd(Math.ceil(t.length/4)*4,"=");return ew(s)}const S2=ge("$ZodBase64URL",(e,t)=>{t.pattern??(t.pattern=Kx),Ot.init(e,t),e._zod.bag.contentEncoding="base64url",e._zod.check=s=>{w2(s.value)||s.issues.push({code:"invalid_format",format:"base64url",input:s.value,inst:e,continue:!t.abort})}}),k2=ge("$ZodE164",(e,t)=>{t.pattern??(t.pattern=zE),Ot.init(e,t)});function j2(e,t=null){try{const s=e.split(".");if(s.length!==3)return!1;const[a]=s;if(!a)return!1;const o=JSON.parse(atob(a));return!("typ"in o&&o?.typ!=="JWT"||!o.alg||t&&(!("alg"in o)||o.alg!==t))}catch{return!1}}const N2=ge("$ZodJWT",(e,t)=>{Ot.init(e,t),e._zod.check=s=>{j2(s.value,t.alg)||s.issues.push({code:"invalid_format",format:"jwt",input:s.value,inst:e,continue:!t.abort})}}),tw=ge("$ZodNumber",(e,t)=>{qt.init(e,t),e._zod.pattern=e._zod.bag.pattern??Fx,e._zod.parse=(s,a)=>{if(t.coerce)try{s.value=Number(s.value)}catch{}const o=s.value;if(typeof o=="number"&&!Number.isNaN(o)&&Number.isFinite(o))return s;const l=typeof o=="number"?Number.isNaN(o)?"NaN":Number.isFinite(o)?void 0:"Infinity":void 0;return s.issues.push({expected:"number",code:"invalid_type",input:o,inst:e,...l?{received:l}:{}}),s}}),E2=ge("$ZodNumberFormat",(e,t)=>{ZE.init(e,t),tw.init(e,t)}),T2=ge("$ZodBoolean",(e,t)=>{qt.init(e,t),e._zod.pattern=UE,e._zod.parse=(s,a)=>{if(t.coerce)try{s.value=!!s.value}catch{}const o=s.value;return typeof o=="boolean"||s.issues.push({expected:"boolean",code:"invalid_type",input:o,inst:e}),s}}),C2=ge("$ZodUnknown",(e,t)=>{qt.init(e,t),e._zod.parse=s=>s}),A2=ge("$ZodNever",(e,t)=>{qt.init(e,t),e._zod.parse=(s,a)=>(s.issues.push({expected:"never",code:"invalid_type",input:s.value,inst:e}),s)});function _v(e,t,s){e.issues.length&&t.issues.push(...ir(s,e.issues)),t.value[s]=e.value}const O2=ge("$ZodArray",(e,t)=>{qt.init(e,t),e._zod.parse=(s,a)=>{const o=s.value;if(!Array.isArray(o))return s.issues.push({expected:"array",code:"invalid_type",input:o,inst:e}),s;s.value=Array(o.length);const l=[];for(let c=0;c<o.length;c++){const f=o[c],m=t.element._zod.run({value:f,issues:[]},a);m instanceof Promise?l.push(m.then(p=>_v(p,s,c))):_v(m,s,c)}return l.length?Promise.all(l).then(()=>s):s}});function td(e,t,s,a,o,l){const c=s in a;if(e.issues.length){if(o&&l&&!c)return;t.issues.push(...ir(s,e.issues))}if(!c&&!o){e.issues.length||t.issues.push({code:"invalid_type",expected:"nonoptional",input:void 0,path:[s]});return}e.value===void 0?c&&(t.value[s]=void 0):t.value[s]=e.value}function nw(e){const t=Object.keys(e.shape);for(const a of t)if(!e.shape?.[a]?._zod?.traits?.has("$ZodType"))throw new Error(`Invalid element at key "${a}": expected a Zod schema`);const s=GN(e.shape);return{...e,keys:t,keySet:new Set(t),numKeys:t.length,optionalKeys:new Set(s)}}function sw(e,t,s,a,o,l){const c=[],f=o.keySet,m=o.catchall._zod,p=m.def.type,_=m.optin==="optional",y=m.optout==="optional";for(const b in t){if(b==="__proto__"||f.has(b))continue;if(p==="never"){c.push(b);continue}const v=m.run({value:t[b],issues:[]},a);v instanceof Promise?e.push(v.then(S=>td(S,s,b,t,_,y))):td(v,s,b,t,_,y)}return c.length&&s.issues.push({code:"unrecognized_keys",keys:c,input:t,inst:l}),e.length?Promise.all(e).then(()=>s):s}const M2=ge("$ZodObject",(e,t)=>{if(qt.init(e,t),!Object.getOwnPropertyDescriptor(t,"shape")?.get){const f=t.shape;Object.defineProperty(t,"shape",{get:()=>{const m={...f};return Object.defineProperty(t,"shape",{value:m}),m}})}const a=Nh(()=>nw(t));st(e._zod,"propValues",()=>{const f=t.shape,m={};for(const p in f){const _=f[p]._zod;if(_.values){m[p]??(m[p]=new Set);for(const y of _.values)m[p].add(y)}}return m});const o=ed,l=t.catchall;let c;e._zod.parse=(f,m)=>{c??(c=a.value);const p=f.value;if(!o(p))return f.issues.push({expected:"object",code:"invalid_type",input:p,inst:e}),f;f.value={};const _=[],y=c.shape;for(const b of c.keys){const v=y[b],S=v._zod.optin==="optional",w=v._zod.optout==="optional",k=v._zod.run({value:p[b],issues:[]},m);k instanceof Promise?_.push(k.then(j=>td(j,f,b,p,S,w))):td(k,f,b,p,S,w)}return l?sw(_,p,f,m,a.value,e):_.length?Promise.all(_).then(()=>f):f}}),z2=ge("$ZodObjectJIT",(e,t)=>{M2.init(e,t);const s=e._zod.parse,a=Nh(()=>nw(t)),o=b=>{const v=new e2(["shape","payload","ctx"]),S=a.value,w=N=>{const T=gv(N);return`shape[${T}]._zod.run({ value: input[${T}], issues: [] }, ctx)`};v.write("const input = payload.value;");const k=Object.create(null);let j=0;for(const N of S.keys)k[N]=`key_${j++}`;v.write("const newResult = {};");for(const N of S.keys){const T=k[N],E=gv(N),O=b[N],R=O?._zod?.optin==="optional",q=O?._zod?.optout==="optional";v.write(`const ${T} = ${w(N)};`),R&&q?v.write(`
        if (${T}.issues.length) {
          if (${E} in input) {
            payload.issues = payload.issues.concat(${T}.issues.map(iss => ({
              ...iss,
              path: iss.path ? [${E}, ...iss.path] : [${E}]
            })));
          }
        }
        
        if (${T}.value === undefined) {
          if (${E} in input) {
            newResult[${E}] = undefined;
          }
        } else {
          newResult[${E}] = ${T}.value;
        }
        
      `):R?v.write(`
        if (${T}.issues.length) {
          payload.issues = payload.issues.concat(${T}.issues.map(iss => ({
            ...iss,
            path: iss.path ? [${E}, ...iss.path] : [${E}]
          })));
        }
        
        if (${T}.value === undefined) {
          if (${E} in input) {
            newResult[${E}] = undefined;
          }
        } else {
          newResult[${E}] = ${T}.value;
        }
        
      `):v.write(`
        const ${T}_present = ${E} in input;
        if (${T}.issues.length) {
          payload.issues = payload.issues.concat(${T}.issues.map(iss => ({
            ...iss,
            path: iss.path ? [${E}, ...iss.path] : [${E}]
          })));
        }
        if (!${T}_present && !${T}.issues.length) {
          payload.issues.push({
            code: "invalid_type",
            expected: "nonoptional",
            input: undefined,
            path: [${E}]
          });
        }

        if (${T}_present) {
          if (${T}.value === undefined) {
            newResult[${E}] = undefined;
          } else {
            newResult[${E}] = ${T}.value;
          }
        }

      `)}v.write("payload.value = newResult;"),v.write("return payload;");const A=v.compile();return(N,T)=>A(b,N,T)};let l;const c=ed,f=!jh.jitless,p=f&&VN.value,_=t.catchall;let y;e._zod.parse=(b,v)=>{y??(y=a.value);const S=b.value;return c(S)?f&&p&&v?.async===!1&&v.jitless!==!0?(l||(l=o(t.shape)),b=l(b,v),_?sw([],S,b,v,y,e):b):s(b,v):(b.issues.push({expected:"object",code:"invalid_type",input:S,inst:e}),b)}});function bv(e,t,s,a){for(const l of e)if(l.issues.length===0)return t.value=l.value,t;const o=e.filter(l=>!sr(l));return o.length===1?(t.value=o[0].value,o[0]):(t.issues.push({code:"invalid_union",input:t.value,inst:s,errors:e.map(l=>l.issues.map(c=>Oi(c,a,Ai())))}),t)}const D2=ge("$ZodUnion",(e,t)=>{qt.init(e,t),st(e._zod,"optin",()=>t.options.some(a=>a._zod.optin==="optional")?"optional":void 0),st(e._zod,"optout",()=>t.options.some(a=>a._zod.optout==="optional")?"optional":void 0),st(e._zod,"values",()=>{if(t.options.every(a=>a._zod.values))return new Set(t.options.flatMap(a=>Array.from(a._zod.values)))}),st(e._zod,"pattern",()=>{if(t.options.every(a=>a._zod.pattern)){const a=t.options.map(o=>o._zod.pattern);return new RegExp(`^(${a.map(o=>Th(o.source)).join("|")})$`)}});const s=t.options.length===1?t.options[0]._zod.run:null;e._zod.parse=(a,o)=>{if(s)return s(a,o);let l=!1;const c=[];for(const f of t.options){const m=f._zod.run({value:a.value,issues:[]},o);if(m instanceof Promise)c.push(m),l=!0;else{if(m.issues.length===0)return m;c.push(m)}}return l?Promise.all(c).then(f=>bv(f,a,e,o)):bv(c,a,e,o)}}),$2=ge("$ZodIntersection",(e,t)=>{qt.init(e,t),e._zod.parse=(s,a)=>{const o=s.value,l=t.left._zod.run({value:o,issues:[]},a),c=t.right._zod.run({value:o,issues:[]},a);return l instanceof Promise||c instanceof Promise?Promise.all([l,c]).then(([m,p])=>vv(s,m,p)):vv(s,l,c)}});function Gm(e,t){if(e===t)return{valid:!0,data:e};if(e instanceof Date&&t instanceof Date&&+e==+t)return{valid:!0,data:e};if(_r(e)&&_r(t)){const s=Object.keys(t),a=Object.keys(e).filter(l=>s.indexOf(l)!==-1),o={...e,...t};for(const l of a){const c=Gm(e[l],t[l]);if(!c.valid)return{valid:!1,mergeErrorPath:[l,...c.mergeErrorPath]};o[l]=c.data}return{valid:!0,data:o}}if(Array.isArray(e)&&Array.isArray(t)){if(e.length!==t.length)return{valid:!1,mergeErrorPath:[]};const s=[];for(let a=0;a<e.length;a++){const o=e[a],l=t[a],c=Gm(o,l);if(!c.valid)return{valid:!1,mergeErrorPath:[a,...c.mergeErrorPath]};s.push(c.data)}return{valid:!0,data:s}}return{valid:!1,mergeErrorPath:[]}}function vv(e,t,s){const a=new Map;let o;for(const f of t.issues)if(f.code==="unrecognized_keys"){o??(o=f);for(const m of f.keys)a.has(m)||a.set(m,{}),a.get(m).l=!0}else e.issues.push(f);for(const f of s.issues)if(f.code==="unrecognized_keys")for(const m of f.keys)a.has(m)||a.set(m,{}),a.get(m).r=!0;else e.issues.push(f);const l=[...a].filter(([,f])=>f.l&&f.r).map(([f])=>f);if(l.length&&o&&e.issues.push({...o,keys:l}),sr(e))return e;const c=Gm(t.value,s.value);if(!c.valid)throw new Error(`Unmergable intersection. Error path: ${JSON.stringify(c.mergeErrorPath)}`);return e.value=c.data,e}const R2=ge("$ZodRecord",(e,t)=>{qt.init(e,t),e._zod.parse=(s,a)=>{const o=s.value;if(!_r(o))return s.issues.push({expected:"record",code:"invalid_type",input:o,inst:e}),s;const l=[],c=t.keyType._zod.values;if(c){s.value={};const f=new Set;for(const p of c)if(typeof p=="string"||typeof p=="number"||typeof p=="symbol"){f.add(typeof p=="number"?p.toString():p);const _=t.keyType._zod.run({value:p,issues:[]},a);if(_ instanceof Promise)throw new Error("Async schemas not supported in object keys currently");if(_.issues.length){s.issues.push({code:"invalid_key",origin:"record",issues:_.issues.map(v=>Oi(v,a,Ai())),input:p,path:[p],inst:e});continue}const y=_.value,b=t.valueType._zod.run({value:o[p],issues:[]},a);b instanceof Promise?l.push(b.then(v=>{v.issues.length&&s.issues.push(...ir(p,v.issues)),s.value[y]=v.value})):(b.issues.length&&s.issues.push(...ir(p,b.issues)),s.value[y]=b.value)}let m;for(const p in o)f.has(p)||(m=m??[],m.push(p));m&&m.length>0&&s.issues.push({code:"unrecognized_keys",input:o,inst:e,keys:m})}else{s.value={};for(const f of Reflect.ownKeys(o)){if(f==="__proto__"||!Object.prototype.propertyIsEnumerable.call(o,f))continue;let m=t.keyType._zod.run({value:f,issues:[]},a);if(m instanceof Promise)throw new Error("Async schemas not supported in object keys currently");if(typeof f=="string"&&Fx.test(f)&&m.issues.length){const y=t.keyType._zod.run({value:Number(f),issues:[]},a);if(y instanceof Promise)throw new Error("Async schemas not supported in object keys currently");y.issues.length===0&&(m=y)}if(m.issues.length){t.mode==="loose"?s.value[f]=o[f]:s.issues.push({code:"invalid_key",origin:"record",issues:m.issues.map(y=>Oi(y,a,Ai())),input:f,path:[f],inst:e});continue}const _=t.valueType._zod.run({value:o[f],issues:[]},a);_ instanceof Promise?l.push(_.then(y=>{y.issues.length&&s.issues.push(...ir(f,y.issues)),s.value[m.value]=y.value})):(_.issues.length&&s.issues.push(...ir(f,_.issues)),s.value[m.value]=_.value)}}return l.length?Promise.all(l).then(()=>s):s}}),L2=ge("$ZodEnum",(e,t)=>{qt.init(e,t);const s=Hx(t.entries),a=new Set(s);e._zod.values=a,e._zod.pattern=new RegExp(`^(${s.filter(o=>KN.has(typeof o)).map(o=>typeof o=="string"?Ed(o):o.toString()).join("|")})$`),e._zod.parse=(o,l)=>{const c=o.value;return a.has(c)||o.issues.push({code:"invalid_value",values:s,input:c,inst:e}),o}}),q2=ge("$ZodTransform",(e,t)=>{qt.init(e,t),e._zod.optin="optional",e._zod.parse=(s,a)=>{if(a.direction==="backward")throw new Bx(e.constructor.name);const o=t.transform(s.value,s);if(a.async)return(o instanceof Promise?o:Promise.resolve(o)).then(c=>(s.value=c,s.fallback=!0,s));if(o instanceof Promise)throw new ur;return s.value=o,s.fallback=!0,s}});function xv(e,t){return t===void 0&&(e.issues.length||e.fallback)?{issues:[],value:void 0}:e}const iw=ge("$ZodOptional",(e,t)=>{qt.init(e,t),e._zod.optin="optional",e._zod.optout="optional",st(e._zod,"values",()=>t.innerType._zod.values?new Set([...t.innerType._zod.values,void 0]):void 0),st(e._zod,"pattern",()=>{const s=t.innerType._zod.pattern;return s?new RegExp(`^(${Th(s.source)})?$`):void 0}),e._zod.parse=(s,a)=>{if(t.innerType._zod.optin==="optional"){const o=s.value,l=t.innerType._zod.run(s,a);return l instanceof Promise?l.then(c=>xv(c,o)):xv(l,o)}return s.value===void 0?s:t.innerType._zod.run(s,a)}}),U2=ge("$ZodExactOptional",(e,t)=>{iw.init(e,t),st(e._zod,"values",()=>t.innerType._zod.values),st(e._zod,"pattern",()=>t.innerType._zod.pattern),e._zod.parse=(s,a)=>t.innerType._zod.run(s,a)}),B2=ge("$ZodNullable",(e,t)=>{qt.init(e,t),st(e._zod,"optin",()=>t.innerType._zod.optin),st(e._zod,"optout",()=>t.innerType._zod.optout),st(e._zod,"pattern",()=>{const s=t.innerType._zod.pattern;return s?new RegExp(`^(${Th(s.source)}|null)$`):void 0}),st(e._zod,"values",()=>t.innerType._zod.values?new Set([...t.innerType._zod.values,null]):void 0),e._zod.parse=(s,a)=>s.value===null?s:t.innerType._zod.run(s,a)}),H2=ge("$ZodDefault",(e,t)=>{qt.init(e,t),e._zod.optin="optional",st(e._zod,"values",()=>t.innerType._zod.values),e._zod.parse=(s,a)=>{if(a.direction==="backward")return t.innerType._zod.run(s,a);if(s.value===void 0)return s.value=t.defaultValue,s;const o=t.innerType._zod.run(s,a);return o instanceof Promise?o.then(l=>wv(l,t)):wv(o,t)}});function wv(e,t){return e.value===void 0&&(e.value=t.defaultValue),e}const I2=ge("$ZodPrefault",(e,t)=>{qt.init(e,t),e._zod.optin="optional",st(e._zod,"values",()=>t.innerType._zod.values),e._zod.parse=(s,a)=>(a.direction==="backward"||s.value===void 0&&(s.value=t.defaultValue),t.innerType._zod.run(s,a))}),Z2=ge("$ZodNonOptional",(e,t)=>{qt.init(e,t),st(e._zod,"values",()=>{const s=t.innerType._zod.values;return s?new Set([...s].filter(a=>a!==void 0)):void 0}),e._zod.parse=(s,a)=>{const o=t.innerType._zod.run(s,a);return o instanceof Promise?o.then(l=>Sv(l,e)):Sv(o,e)}});function Sv(e,t){return!e.issues.length&&e.value===void 0&&e.issues.push({code:"invalid_type",expected:"nonoptional",input:e.value,inst:t}),e}const P2=ge("$ZodCatch",(e,t)=>{qt.init(e,t),e._zod.optin="optional",st(e._zod,"optout",()=>t.innerType._zod.optout),st(e._zod,"values",()=>t.innerType._zod.values),e._zod.parse=(s,a)=>{if(a.direction==="backward")return t.innerType._zod.run(s,a);const o=t.innerType._zod.run(s,a);return o instanceof Promise?o.then(l=>(s.value=l.value,l.issues.length&&(s.value=t.catchValue({...s,error:{issues:l.issues.map(c=>Oi(c,a,Ai()))},input:s.value}),s.issues=[],s.fallback=!0),s)):(s.value=o.value,o.issues.length&&(s.value=t.catchValue({...s,error:{issues:o.issues.map(l=>Oi(l,a,Ai()))},input:s.value}),s.issues=[],s.fallback=!0),s)}}),Y2=ge("$ZodPipe",(e,t)=>{qt.init(e,t),st(e._zod,"values",()=>t.in._zod.values),st(e._zod,"optin",()=>t.in._zod.optin),st(e._zod,"optout",()=>t.out._zod.optout),st(e._zod,"propValues",()=>t.in._zod.propValues),e._zod.parse=(s,a)=>{if(a.direction==="backward"){const l=t.out._zod.run(s,a);return l instanceof Promise?l.then(c=>ju(c,t.in,a)):ju(l,t.in,a)}const o=t.in._zod.run(s,a);return o instanceof Promise?o.then(l=>ju(l,t.out,a)):ju(o,t.out,a)}});function ju(e,t,s){return e.issues.length?(e.aborted=!0,e):t._zod.run({value:e.value,issues:e.issues,fallback:e.fallback},s)}const V2=ge("$ZodReadonly",(e,t)=>{qt.init(e,t),st(e._zod,"propValues",()=>t.innerType._zod.propValues),st(e._zod,"values",()=>t.innerType._zod.values),st(e._zod,"optin",()=>t.innerType?._zod?.optin),st(e._zod,"optout",()=>t.innerType?._zod?.optout),e._zod.parse=(s,a)=>{if(a.direction==="backward")return t.innerType._zod.run(s,a);const o=t.innerType._zod.run(s,a);return o instanceof Promise?o.then(kv):kv(o)}});function kv(e){return e.value=Object.freeze(e.value),e}const K2=ge("$ZodLazy",(e,t)=>{qt.init(e,t),st(e._zod,"innerType",()=>{const s=t;return s._cachedInner||(s._cachedInner=t.getter()),s._cachedInner}),st(e._zod,"pattern",()=>e._zod.innerType?._zod?.pattern),st(e._zod,"propValues",()=>e._zod.innerType?._zod?.propValues),st(e._zod,"optin",()=>e._zod.innerType?._zod?.optin??void 0),st(e._zod,"optout",()=>e._zod.innerType?._zod?.optout??void 0),e._zod.parse=(s,a)=>e._zod.innerType._zod.run(s,a)}),G2=ge("$ZodCustom",(e,t)=>{Yn.init(e,t),qt.init(e,t),e._zod.parse=(s,a)=>s,e._zod.check=s=>{const a=s.value,o=t.fn(a);if(o instanceof Promise)return o.then(l=>jv(l,s,a,e));jv(o,s,a,e)}});function jv(e,t,s,a){if(!e){const o={code:"custom",input:s,inst:a,path:[...a._zod.def.path??[]],continue:!a._zod.def.abort};a._zod.def.params&&(o.params=a._zod.def.params),t.issues.push(Ll(o))}}var Nv;class X2{constructor(){this._map=new WeakMap,this._idmap=new Map}add(t,...s){const a=s[0];return this._map.set(t,a),a&&typeof a=="object"&&"id"in a&&this._idmap.set(a.id,t),this}clear(){return this._map=new WeakMap,this._idmap=new Map,this}remove(t){const s=this._map.get(t);return s&&typeof s=="object"&&"id"in s&&this._idmap.delete(s.id),this._map.delete(t),this}get(t){const s=t._zod.parent;if(s){const a={...this.get(s)??{}};delete a.id;const o={...a,...this._map.get(t)};return Object.keys(o).length?o:void 0}return this._map.get(t)}has(t){return this._map.has(t)}}function F2(){return new X2}(Nv=globalThis).__zod_globalRegistry??(Nv.__zod_globalRegistry=F2());const Tl=globalThis.__zod_globalRegistry;function J2(e,t){return new e({type:"string",...De(t)})}function Q2(e,t){return new e({type:"string",format:"email",check:"string_format",abort:!1,...De(t)})}function Ev(e,t){return new e({type:"string",format:"guid",check:"string_format",abort:!1,...De(t)})}function W2(e,t){return new e({type:"string",format:"uuid",check:"string_format",abort:!1,...De(t)})}function eT(e,t){return new e({type:"string",format:"uuid",check:"string_format",abort:!1,version:"v4",...De(t)})}function tT(e,t){return new e({type:"string",format:"uuid",check:"string_format",abort:!1,version:"v6",...De(t)})}function nT(e,t){return new e({type:"string",format:"uuid",check:"string_format",abort:!1,version:"v7",...De(t)})}function sT(e,t){return new e({type:"string",format:"url",check:"string_format",abort:!1,...De(t)})}function iT(e,t){return new e({type:"string",format:"emoji",check:"string_format",abort:!1,...De(t)})}function aT(e,t){return new e({type:"string",format:"nanoid",check:"string_format",abort:!1,...De(t)})}function oT(e,t){return new e({type:"string",format:"cuid",check:"string_format",abort:!1,...De(t)})}function rT(e,t){return new e({type:"string",format:"cuid2",check:"string_format",abort:!1,...De(t)})}function lT(e,t){return new e({type:"string",format:"ulid",check:"string_format",abort:!1,...De(t)})}function cT(e,t){return new e({type:"string",format:"xid",check:"string_format",abort:!1,...De(t)})}function uT(e,t){return new e({type:"string",format:"ksuid",check:"string_format",abort:!1,...De(t)})}function dT(e,t){return new e({type:"string",format:"ipv4",check:"string_format",abort:!1,...De(t)})}function fT(e,t){return new e({type:"string",format:"ipv6",check:"string_format",abort:!1,...De(t)})}function pT(e,t){return new e({type:"string",format:"cidrv4",check:"string_format",abort:!1,...De(t)})}function mT(e,t){return new e({type:"string",format:"cidrv6",check:"string_format",abort:!1,...De(t)})}function hT(e,t){return new e({type:"string",format:"base64",check:"string_format",abort:!1,...De(t)})}function gT(e,t){return new e({type:"string",format:"base64url",check:"string_format",abort:!1,...De(t)})}function yT(e,t){return new e({type:"string",format:"e164",check:"string_format",abort:!1,...De(t)})}function _T(e,t){return new e({type:"string",format:"jwt",check:"string_format",abort:!1,...De(t)})}function bT(e,t){return new e({type:"string",format:"datetime",check:"string_format",offset:!1,local:!1,precision:null,...De(t)})}function vT(e,t){return new e({type:"string",format:"date",check:"string_format",...De(t)})}function xT(e,t){return new e({type:"string",format:"time",check:"string_format",precision:null,...De(t)})}function wT(e,t){return new e({type:"string",format:"duration",check:"string_format",...De(t)})}function ST(e,t){return new e({type:"number",checks:[],...De(t)})}function kT(e,t){return new e({type:"number",check:"number_format",abort:!1,format:"safeint",...De(t)})}function jT(e,t){return new e({type:"boolean",...De(t)})}function NT(e){return new e({type:"unknown"})}function ET(e,t){return new e({type:"never",...De(t)})}function Tv(e,t){return new Qx({check:"less_than",...De(t),value:e,inclusive:!1})}function pm(e,t){return new Qx({check:"less_than",...De(t),value:e,inclusive:!0})}function Cv(e,t){return new Wx({check:"greater_than",...De(t),value:e,inclusive:!1})}function mm(e,t){return new Wx({check:"greater_than",...De(t),value:e,inclusive:!0})}function Av(e,t){return new IE({check:"multiple_of",...De(t),value:e})}function aw(e,t){return new PE({check:"max_length",...De(t),maximum:e})}function nd(e,t){return new YE({check:"min_length",...De(t),minimum:e})}function ow(e,t){return new VE({check:"length_equals",...De(t),length:e})}function TT(e,t){return new KE({check:"string_format",format:"regex",...De(t),pattern:e})}function CT(e){return new GE({check:"string_format",format:"lowercase",...De(e)})}function AT(e){return new XE({check:"string_format",format:"uppercase",...De(e)})}function OT(e,t){return new FE({check:"string_format",format:"includes",...De(t),includes:e})}function MT(e,t){return new JE({check:"string_format",format:"starts_with",...De(t),prefix:e})}function zT(e,t){return new QE({check:"string_format",format:"ends_with",...De(t),suffix:e})}function Or(e){return new WE({check:"overwrite",tx:e})}function DT(e){return Or(t=>t.normalize(e))}function $T(){return Or(e=>e.trim())}function RT(){return Or(e=>e.toLowerCase())}function LT(){return Or(e=>e.toUpperCase())}function qT(){return Or(e=>YN(e))}function UT(e,t,s){return new e({type:"array",element:t,...De(s)})}function BT(e,t,s){return new e({type:"custom",check:"custom",fn:t,...De(s)})}function HT(e,t){const s=IT(a=>(a.addIssue=o=>{if(typeof o=="string")a.issues.push(Ll(o,a.value,s._zod.def));else{const l=o;l.fatal&&(l.continue=!1),l.code??(l.code="custom"),l.input??(l.input=a.value),l.inst??(l.inst=s),l.continue??(l.continue=!s._zod.def.abort),a.issues.push(Ll(l))}},e(a.value,a)),t);return s}function IT(e,t){const s=new Yn({check:"custom",...De(t)});return s._zod.check=e,s}function rw(e){let t=e?.target??"draft-2020-12";return t==="draft-4"&&(t="draft-04"),t==="draft-7"&&(t="draft-07"),{processors:e.processors??{},metadataRegistry:e?.metadata??Tl,target:t,unrepresentable:e?.unrepresentable??"throw",override:e?.override??(()=>{}),io:e?.io??"output",counter:0,seen:new Map,cycles:e?.cycles??"ref",reused:e?.reused??"inline",external:e?.external??void 0}}function Gt(e,t,s={path:[],schemaPath:[]}){var a;const o=e._zod.def,l=t.seen.get(e);if(l)return l.count++,s.schemaPath.includes(e)&&(l.cycle=s.path),l.schema;const c={schema:{},count:1,cycle:void 0,path:s.path};t.seen.set(e,c);const f=e._zod.toJSONSchema?.();if(f)c.schema=f;else{const _={...s,schemaPath:[...s.schemaPath,e],path:s.path};if(e._zod.processJSONSchema)e._zod.processJSONSchema(t,c.schema,_);else{const b=c.schema,v=t.processors[o.type];if(!v)throw new Error(`[toJSONSchema]: Non-representable type encountered: ${o.type}`);v(e,t,b,_)}const y=e._zod.parent;y&&(c.ref||(c.ref=y),Gt(y,t,_),t.seen.get(y).isParent=!0)}const m=t.metadataRegistry.get(e);return m&&Object.assign(c.schema,m),t.io==="input"&&On(e)&&(delete c.schema.examples,delete c.schema.default),t.io==="input"&&"_prefault"in c.schema&&((a=c.schema).default??(a.default=c.schema._prefault)),delete c.schema._prefault,t.seen.get(e).schema}function lw(e,t){const s=e.seen.get(t);if(!s)throw new Error("Unprocessed schema. This is a bug in Zod.");const a=new Map;for(const c of e.seen.entries()){const f=e.metadataRegistry.get(c[0])?.id;if(f){const m=a.get(f);if(m&&m!==c[0])throw new Error(`Duplicate schema id "${f}" detected during JSON Schema conversion. Two different schemas cannot share the same id when converted together.`);a.set(f,c[0])}}const o=c=>{const f=e.target==="draft-2020-12"?"$defs":"definitions";if(e.external){const y=e.external.registry.get(c[0])?.id,b=e.external.uri??(S=>S);if(y)return{ref:b(y)};const v=c[1].defId??c[1].schema.id??`schema${e.counter++}`;return c[1].defId=v,{defId:v,ref:`${b("__shared")}#/${f}/${v}`}}if(c[1]===s)return{ref:"#"};const p=`#/${f}/`,_=c[1].schema.id??`__schema${e.counter++}`;return{defId:_,ref:p+_}},l=c=>{if(c[1].schema.$ref)return;const f=c[1],{ref:m,defId:p}=o(c);f.def={...f.schema},p&&(f.defId=p);const _=f.schema;for(const y in _)delete _[y];_.$ref=m};if(e.cycles==="throw")for(const c of e.seen.entries()){const f=c[1];if(f.cycle)throw new Error(`Cycle detected: #/${f.cycle?.join("/")}/<root>

Set the \`cycles\` parameter to \`"ref"\` to resolve cyclical schemas with defs.`)}for(const c of e.seen.entries()){const f=c[1];if(t===c[0]){l(c);continue}if(e.external){const p=e.external.registry.get(c[0])?.id;if(t!==c[0]&&p){l(c);continue}}if(e.metadataRegistry.get(c[0])?.id){l(c);continue}if(f.cycle){l(c);continue}if(f.count>1&&e.reused==="ref"){l(c);continue}}}function cw(e,t){const s=e.seen.get(t);if(!s)throw new Error("Unprocessed schema. This is a bug in Zod.");const a=f=>{const m=e.seen.get(f);if(m.ref===null)return;const p=m.def??m.schema,_={...p},y=m.ref;if(m.ref=null,y){a(y);const v=e.seen.get(y),S=v.schema;if(S.$ref&&(e.target==="draft-07"||e.target==="draft-04"||e.target==="openapi-3.0")?(p.allOf=p.allOf??[],p.allOf.push(S)):Object.assign(p,S),Object.assign(p,_),f._zod.parent===y)for(const k in p)k==="$ref"||k==="allOf"||k in _||delete p[k];if(S.$ref&&v.def)for(const k in p)k==="$ref"||k==="allOf"||k in v.def&&JSON.stringify(p[k])===JSON.stringify(v.def[k])&&delete p[k]}const b=f._zod.parent;if(b&&b!==y){a(b);const v=e.seen.get(b);if(v?.schema.$ref&&(p.$ref=v.schema.$ref,v.def))for(const S in p)S==="$ref"||S==="allOf"||S in v.def&&JSON.stringify(p[S])===JSON.stringify(v.def[S])&&delete p[S]}e.override({zodSchema:f,jsonSchema:p,path:m.path??[]})};for(const f of[...e.seen.entries()].reverse())a(f[0]);const o={};if(e.target==="draft-2020-12"?o.$schema="https://json-schema.org/draft/2020-12/schema":e.target==="draft-07"?o.$schema="http://json-schema.org/draft-07/schema#":e.target==="draft-04"?o.$schema="http://json-schema.org/draft-04/schema#":e.target,e.external?.uri){const f=e.external.registry.get(t)?.id;if(!f)throw new Error("Schema is missing an `id` property");o.$id=e.external.uri(f)}Object.assign(o,s.def??s.schema);const l=e.metadataRegistry.get(t)?.id;l!==void 0&&o.id===l&&delete o.id;const c=e.external?.defs??{};for(const f of e.seen.entries()){const m=f[1];m.def&&m.defId&&(m.def.id===m.defId&&delete m.def.id,c[m.defId]=m.def)}e.external||Object.keys(c).length>0&&(e.target==="draft-2020-12"?o.$defs=c:o.definitions=c);try{const f=JSON.parse(JSON.stringify(o));return Object.defineProperty(f,"~standard",{value:{...t["~standard"],jsonSchema:{input:sd(t,"input",e.processors),output:sd(t,"output",e.processors)}},enumerable:!1,writable:!1}),f}catch{throw new Error("Error converting schema to JSON.")}}function On(e,t){const s=t??{seen:new Set};if(s.seen.has(e))return!1;s.seen.add(e);const a=e._zod.def;if(a.type==="transform")return!0;if(a.type==="array")return On(a.element,s);if(a.type==="set")return On(a.valueType,s);if(a.type==="lazy")return On(a.getter(),s);if(a.type==="promise"||a.type==="optional"||a.type==="nonoptional"||a.type==="nullable"||a.type==="readonly"||a.type==="default"||a.type==="prefault")return On(a.innerType,s);if(a.type==="intersection")return On(a.left,s)||On(a.right,s);if(a.type==="record"||a.type==="map")return On(a.keyType,s)||On(a.valueType,s);if(a.type==="pipe")return e._zod.traits.has("$ZodCodec")?!0:On(a.in,s)||On(a.out,s);if(a.type==="object"){for(const o in a.shape)if(On(a.shape[o],s))return!0;return!1}if(a.type==="union"){for(const o of a.options)if(On(o,s))return!0;return!1}if(a.type==="tuple"){for(const o of a.items)if(On(o,s))return!0;return!!(a.rest&&On(a.rest,s))}return!1}const ZT=(e,t={})=>s=>{const a=rw({...s,processors:t});return Gt(e,a),lw(a,e),cw(a,e)},sd=(e,t,s={})=>a=>{const{libraryOptions:o,target:l}=a??{},c=rw({...o??{},target:l,io:t,processors:s});return Gt(e,c),lw(c,e),cw(c,e)},PT={guid:"uuid",url:"uri",datetime:"date-time",json_string:"json-string",regex:""},YT=(e,t,s,a)=>{const o=s;o.type="string";const{minimum:l,maximum:c,format:f,patterns:m,contentEncoding:p}=e._zod.bag;if(typeof l=="number"&&(o.minLength=l),typeof c=="number"&&(o.maxLength=c),f&&(o.format=PT[f]??f,o.format===""&&delete o.format,f==="time"&&delete o.format),p&&(o.contentEncoding=p),m&&m.size>0){const _=[...m];_.length===1?o.pattern=_[0].source:_.length>1&&(o.allOf=[..._.map(y=>({...t.target==="draft-07"||t.target==="draft-04"||t.target==="openapi-3.0"?{type:"string"}:{},pattern:y.source}))])}},VT=(e,t,s,a)=>{const o=s,{minimum:l,maximum:c,format:f,multipleOf:m,exclusiveMaximum:p,exclusiveMinimum:_}=e._zod.bag;typeof f=="string"&&f.includes("int")?o.type="integer":o.type="number";const y=typeof _=="number"&&_>=(l??Number.NEGATIVE_INFINITY),b=typeof p=="number"&&p<=(c??Number.POSITIVE_INFINITY),v=t.target==="draft-04"||t.target==="openapi-3.0";y?v?(o.minimum=_,o.exclusiveMinimum=!0):o.exclusiveMinimum=_:typeof l=="number"&&(o.minimum=l),b?v?(o.maximum=p,o.exclusiveMaximum=!0):o.exclusiveMaximum=p:typeof c=="number"&&(o.maximum=c),typeof m=="number"&&(o.multipleOf=m)},KT=(e,t,s,a)=>{s.type="boolean"},GT=(e,t,s,a)=>{s.not={}},XT=(e,t,s,a)=>{},FT=(e,t,s,a)=>{const o=e._zod.def,l=Hx(o.entries);l.every(c=>typeof c=="number")&&(s.type="number"),l.every(c=>typeof c=="string")&&(s.type="string"),s.enum=l},JT=(e,t,s,a)=>{if(t.unrepresentable==="throw")throw new Error("Custom types cannot be represented in JSON Schema")},QT=(e,t,s,a)=>{if(t.unrepresentable==="throw")throw new Error("Transforms cannot be represented in JSON Schema")},WT=(e,t,s,a)=>{const o=s,l=e._zod.def,{minimum:c,maximum:f}=e._zod.bag;typeof c=="number"&&(o.minItems=c),typeof f=="number"&&(o.maxItems=f),o.type="array",o.items=Gt(l.element,t,{...a,path:[...a.path,"items"]})},eC=(e,t,s,a)=>{const o=s,l=e._zod.def;o.type="object",o.properties={};const c=l.shape;for(const p in c)o.properties[p]=Gt(c[p],t,{...a,path:[...a.path,"properties",p]});const f=new Set(Object.keys(c)),m=new Set([...f].filter(p=>{const _=l.shape[p]._zod;return t.io==="input"?_.optin===void 0:_.optout===void 0}));m.size>0&&(o.required=Array.from(m)),l.catchall?._zod.def.type==="never"?o.additionalProperties=!1:l.catchall?l.catchall&&(o.additionalProperties=Gt(l.catchall,t,{...a,path:[...a.path,"additionalProperties"]})):t.io==="output"&&(o.additionalProperties=!1)},tC=(e,t,s,a)=>{const o=e._zod.def,l=o.inclusive===!1,c=o.options.map((f,m)=>Gt(f,t,{...a,path:[...a.path,l?"oneOf":"anyOf",m]}));l?s.oneOf=c:s.anyOf=c},nC=(e,t,s,a)=>{const o=e._zod.def,l=Gt(o.left,t,{...a,path:[...a.path,"allOf",0]}),c=Gt(o.right,t,{...a,path:[...a.path,"allOf",1]}),f=p=>"allOf"in p&&Object.keys(p).length===1,m=[...f(l)?l.allOf:[l],...f(c)?c.allOf:[c]];s.allOf=m},sC=(e,t,s,a)=>{const o=s,l=e._zod.def;o.type="object";const c=l.keyType,m=c._zod.bag?.patterns;if(l.mode==="loose"&&m&&m.size>0){const _=Gt(l.valueType,t,{...a,path:[...a.path,"patternProperties","*"]});o.patternProperties={};for(const y of m)o.patternProperties[y.source]=_}else(t.target==="draft-07"||t.target==="draft-2020-12")&&(o.propertyNames=Gt(l.keyType,t,{...a,path:[...a.path,"propertyNames"]})),o.additionalProperties=Gt(l.valueType,t,{...a,path:[...a.path,"additionalProperties"]});const p=c._zod.values;if(p){const _=[...p].filter(y=>typeof y=="string"||typeof y=="number");_.length>0&&(o.required=_)}},iC=(e,t,s,a)=>{const o=e._zod.def,l=Gt(o.innerType,t,a),c=t.seen.get(e);t.target==="openapi-3.0"?(c.ref=o.innerType,s.nullable=!0):s.anyOf=[l,{type:"null"}]},aC=(e,t,s,a)=>{const o=e._zod.def;Gt(o.innerType,t,a);const l=t.seen.get(e);l.ref=o.innerType},oC=(e,t,s,a)=>{const o=e._zod.def;Gt(o.innerType,t,a);const l=t.seen.get(e);l.ref=o.innerType,s.default=JSON.parse(JSON.stringify(o.defaultValue))},rC=(e,t,s,a)=>{const o=e._zod.def;Gt(o.innerType,t,a);const l=t.seen.get(e);l.ref=o.innerType,t.io==="input"&&(s._prefault=JSON.parse(JSON.stringify(o.defaultValue)))},lC=(e,t,s,a)=>{const o=e._zod.def;Gt(o.innerType,t,a);const l=t.seen.get(e);l.ref=o.innerType;let c;try{c=o.catchValue(void 0)}catch{throw new Error("Dynamic catch values are not supported in JSON Schema")}s.default=c},cC=(e,t,s,a)=>{const o=e._zod.def,l=o.in._zod.traits.has("$ZodTransform"),c=t.io==="input"?l?o.out:o.in:o.out;Gt(c,t,a);const f=t.seen.get(e);f.ref=c},uC=(e,t,s,a)=>{const o=e._zod.def;Gt(o.innerType,t,a);const l=t.seen.get(e);l.ref=o.innerType,s.readOnly=!0},uw=(e,t,s,a)=>{const o=e._zod.def;Gt(o.innerType,t,a);const l=t.seen.get(e);l.ref=o.innerType},dC=(e,t,s,a)=>{const o=e._zod.innerType;Gt(o,t,a);const l=t.seen.get(e);l.ref=o},fC=ge("ZodISODateTime",(e,t)=>{p2.init(e,t),Bt.init(e,t)});function pC(e){return bT(fC,e)}const mC=ge("ZodISODate",(e,t)=>{m2.init(e,t),Bt.init(e,t)});function hC(e){return vT(mC,e)}const gC=ge("ZodISOTime",(e,t)=>{h2.init(e,t),Bt.init(e,t)});function yC(e){return xT(gC,e)}const _C=ge("ZodISODuration",(e,t)=>{g2.init(e,t),Bt.init(e,t)});function bC(e){return wT(_C,e)}const vC=(e,t)=>{Yx.init(e,t),e.name="ZodError",Object.defineProperties(e,{format:{value:s=>aE(e,s)},flatten:{value:s=>iE(e,s)},addIssue:{value:s=>{e.issues.push(s),e.message=JSON.stringify(e.issues,Km,2)}},addIssues:{value:s=>{e.issues.push(...s),e.message=JSON.stringify(e.issues,Km,2)}},isEmpty:{get(){return e.issues.length===0}}})},bs=ge("ZodError",vC,{Parent:Error}),xC=Ah(bs),wC=Oh(bs),SC=Td(bs),kC=Cd(bs),jC=lE(bs),NC=cE(bs),EC=uE(bs),TC=dE(bs),CC=fE(bs),AC=pE(bs),OC=mE(bs),MC=hE(bs),Ov=new WeakMap;function ec(e,t,s){const a=Object.getPrototypeOf(e);let o=Ov.get(a);if(o||(o=new Set,Ov.set(a,o)),!o.has(t)){o.add(t);for(const l in s){const c=s[l];Object.defineProperty(a,l,{configurable:!0,enumerable:!1,get(){const f=c.bind(this);return Object.defineProperty(this,l,{configurable:!0,writable:!0,enumerable:!0,value:f}),f},set(f){Object.defineProperty(this,l,{configurable:!0,writable:!0,enumerable:!0,value:f})}})}}}const Ut=ge("ZodType",(e,t)=>(qt.init(e,t),Object.assign(e["~standard"],{jsonSchema:{input:sd(e,"input"),output:sd(e,"output")}}),e.toJSONSchema=ZT(e,{}),e.def=t,e.type=t.type,Object.defineProperty(e,"_def",{value:t}),e.parse=(s,a)=>xC(e,s,a,{callee:e.parse}),e.safeParse=(s,a)=>SC(e,s,a),e.parseAsync=async(s,a)=>wC(e,s,a,{callee:e.parseAsync}),e.safeParseAsync=async(s,a)=>kC(e,s,a),e.spa=e.safeParseAsync,e.encode=(s,a)=>jC(e,s,a),e.decode=(s,a)=>NC(e,s,a),e.encodeAsync=async(s,a)=>EC(e,s,a),e.decodeAsync=async(s,a)=>TC(e,s,a),e.safeEncode=(s,a)=>CC(e,s,a),e.safeDecode=(s,a)=>AC(e,s,a),e.safeEncodeAsync=async(s,a)=>OC(e,s,a),e.safeDecodeAsync=async(s,a)=>MC(e,s,a),ec(e,"ZodType",{check(...s){const a=this.def;return this.clone(ga(a,{checks:[...a.checks??[],...s.map(o=>typeof o=="function"?{_zod:{check:o,def:{check:"custom"},onattach:[]}}:o)]}),{parent:!0})},with(...s){return this.check(...s)},clone(s,a){return ya(this,s,a)},brand(){return this},register(s,a){return s.add(this,a),this},refine(s,a){return this.check(EA(s,a))},superRefine(s,a){return this.check(TA(s,a))},overwrite(s){return this.check(Or(s))},optional(){return $v(this)},exactOptional(){return fA(this)},nullable(){return Rv(this)},nullish(){return $v(Rv(this))},nonoptional(s){return _A(this,s)},array(){return dt(this)},or(s){return aA([this,s])},and(s){return rA(this,s)},transform(s){return Lv(this,uA(s))},default(s){return hA(this,s)},prefault(s){return yA(this,s)},catch(s){return vA(this,s)},pipe(s){return Lv(this,s)},readonly(){return SA(this)},describe(s){const a=this.clone();return Tl.add(a,{description:s}),a},meta(...s){if(s.length===0)return Tl.get(this);const a=this.clone();return Tl.add(a,s[0]),a},isOptional(){return this.safeParse(void 0).success},isNullable(){return this.safeParse(null).success},apply(s){return s(this)}}),Object.defineProperty(e,"description",{get(){return Tl.get(e)?.description},configurable:!0}),e)),dw=ge("_ZodString",(e,t)=>{Mh.init(e,t),Ut.init(e,t),e._zod.processJSONSchema=(a,o,l)=>YT(e,a,o);const s=e._zod.bag;e.format=s.format??null,e.minLength=s.minimum??null,e.maxLength=s.maximum??null,ec(e,"_ZodString",{regex(...a){return this.check(TT(...a))},includes(...a){return this.check(OT(...a))},startsWith(...a){return this.check(MT(...a))},endsWith(...a){return this.check(zT(...a))},min(...a){return this.check(nd(...a))},max(...a){return this.check(aw(...a))},length(...a){return this.check(ow(...a))},nonempty(...a){return this.check(nd(1,...a))},lowercase(a){return this.check(CT(a))},uppercase(a){return this.check(AT(a))},trim(){return this.check($T())},normalize(...a){return this.check(DT(...a))},toLowerCase(){return this.check(RT())},toUpperCase(){return this.check(LT())},slugify(){return this.check(qT())}})}),zC=ge("ZodString",(e,t)=>{Mh.init(e,t),dw.init(e,t),e.email=s=>e.check(Q2(DC,s)),e.url=s=>e.check(sT($C,s)),e.jwt=s=>e.check(_T(FC,s)),e.emoji=s=>e.check(iT(RC,s)),e.guid=s=>e.check(Ev(Mv,s)),e.uuid=s=>e.check(W2(Nu,s)),e.uuidv4=s=>e.check(eT(Nu,s)),e.uuidv6=s=>e.check(tT(Nu,s)),e.uuidv7=s=>e.check(nT(Nu,s)),e.nanoid=s=>e.check(aT(LC,s)),e.guid=s=>e.check(Ev(Mv,s)),e.cuid=s=>e.check(oT(qC,s)),e.cuid2=s=>e.check(rT(UC,s)),e.ulid=s=>e.check(lT(BC,s)),e.base64=s=>e.check(hT(KC,s)),e.base64url=s=>e.check(gT(GC,s)),e.xid=s=>e.check(cT(HC,s)),e.ksuid=s=>e.check(uT(IC,s)),e.ipv4=s=>e.check(dT(ZC,s)),e.ipv6=s=>e.check(fT(PC,s)),e.cidrv4=s=>e.check(pT(YC,s)),e.cidrv6=s=>e.check(mT(VC,s)),e.e164=s=>e.check(yT(XC,s)),e.datetime=s=>e.check(pC(s)),e.date=s=>e.check(hC(s)),e.time=s=>e.check(yC(s)),e.duration=s=>e.check(bC(s))});function ue(e){return J2(zC,e)}const Bt=ge("ZodStringFormat",(e,t)=>{Ot.init(e,t),dw.init(e,t)}),DC=ge("ZodEmail",(e,t)=>{i2.init(e,t),Bt.init(e,t)}),Mv=ge("ZodGUID",(e,t)=>{n2.init(e,t),Bt.init(e,t)}),Nu=ge("ZodUUID",(e,t)=>{s2.init(e,t),Bt.init(e,t)}),$C=ge("ZodURL",(e,t)=>{a2.init(e,t),Bt.init(e,t)}),RC=ge("ZodEmoji",(e,t)=>{o2.init(e,t),Bt.init(e,t)}),LC=ge("ZodNanoID",(e,t)=>{r2.init(e,t),Bt.init(e,t)}),qC=ge("ZodCUID",(e,t)=>{l2.init(e,t),Bt.init(e,t)}),UC=ge("ZodCUID2",(e,t)=>{c2.init(e,t),Bt.init(e,t)}),BC=ge("ZodULID",(e,t)=>{u2.init(e,t),Bt.init(e,t)}),HC=ge("ZodXID",(e,t)=>{d2.init(e,t),Bt.init(e,t)}),IC=ge("ZodKSUID",(e,t)=>{f2.init(e,t),Bt.init(e,t)}),ZC=ge("ZodIPv4",(e,t)=>{y2.init(e,t),Bt.init(e,t)}),PC=ge("ZodIPv6",(e,t)=>{_2.init(e,t),Bt.init(e,t)}),YC=ge("ZodCIDRv4",(e,t)=>{b2.init(e,t),Bt.init(e,t)}),VC=ge("ZodCIDRv6",(e,t)=>{v2.init(e,t),Bt.init(e,t)}),KC=ge("ZodBase64",(e,t)=>{x2.init(e,t),Bt.init(e,t)}),GC=ge("ZodBase64URL",(e,t)=>{S2.init(e,t),Bt.init(e,t)}),XC=ge("ZodE164",(e,t)=>{k2.init(e,t),Bt.init(e,t)}),FC=ge("ZodJWT",(e,t)=>{N2.init(e,t),Bt.init(e,t)}),fw=ge("ZodNumber",(e,t)=>{tw.init(e,t),Ut.init(e,t),e._zod.processJSONSchema=(a,o,l)=>VT(e,a,o),ec(e,"ZodNumber",{gt(a,o){return this.check(Cv(a,o))},gte(a,o){return this.check(mm(a,o))},min(a,o){return this.check(mm(a,o))},lt(a,o){return this.check(Tv(a,o))},lte(a,o){return this.check(pm(a,o))},max(a,o){return this.check(pm(a,o))},int(a){return this.check(zv(a))},safe(a){return this.check(zv(a))},positive(a){return this.check(Cv(0,a))},nonnegative(a){return this.check(mm(0,a))},negative(a){return this.check(Tv(0,a))},nonpositive(a){return this.check(pm(0,a))},multipleOf(a,o){return this.check(Av(a,o))},step(a,o){return this.check(Av(a,o))},finite(){return this}});const s=e._zod.bag;e.minValue=Math.max(s.minimum??Number.NEGATIVE_INFINITY,s.exclusiveMinimum??Number.NEGATIVE_INFINITY)??null,e.maxValue=Math.min(s.maximum??Number.POSITIVE_INFINITY,s.exclusiveMaximum??Number.POSITIVE_INFINITY)??null,e.isInt=(s.format??"").includes("int")||Number.isSafeInteger(s.multipleOf??.5),e.isFinite=!0,e.format=s.format??null});function cn(e){return ST(fw,e)}const JC=ge("ZodNumberFormat",(e,t)=>{E2.init(e,t),fw.init(e,t)});function zv(e){return kT(JC,e)}const QC=ge("ZodBoolean",(e,t)=>{T2.init(e,t),Ut.init(e,t),e._zod.processJSONSchema=(s,a,o)=>KT(e,s,a)});function sn(e){return jT(QC,e)}const WC=ge("ZodUnknown",(e,t)=>{C2.init(e,t),Ut.init(e,t),e._zod.processJSONSchema=(s,a,o)=>XT()});function io(){return NT(WC)}const eA=ge("ZodNever",(e,t)=>{A2.init(e,t),Ut.init(e,t),e._zod.processJSONSchema=(s,a,o)=>GT(e,s,a)});function tA(e){return ET(eA,e)}const nA=ge("ZodArray",(e,t)=>{O2.init(e,t),Ut.init(e,t),e._zod.processJSONSchema=(s,a,o)=>WT(e,s,a,o),e.element=t.element,ec(e,"ZodArray",{min(s,a){return this.check(nd(s,a))},nonempty(s){return this.check(nd(1,s))},max(s,a){return this.check(aw(s,a))},length(s,a){return this.check(ow(s,a))},unwrap(){return this.element}})});function dt(e,t){return UT(nA,e,t)}const sA=ge("ZodObject",(e,t)=>{z2.init(e,t),Ut.init(e,t),e._zod.processJSONSchema=(s,a,o)=>eC(e,s,a,o),st(e,"shape",()=>t.shape),ec(e,"ZodObject",{keyof(){return ao(Object.keys(this._zod.def.shape))},catchall(s){return this.clone({...this._zod.def,catchall:s})},passthrough(){return this.clone({...this._zod.def,catchall:io()})},loose(){return this.clone({...this._zod.def,catchall:io()})},strict(){return this.clone({...this._zod.def,catchall:tA()})},strip(){return this.clone({...this._zod.def,catchall:void 0})},extend(s){return QN(this,s)},safeExtend(s){return WN(this,s)},merge(s){return eE(this,s)},pick(s){return FN(this,s)},omit(s){return JN(this,s)},partial(...s){return tE(pw,this,s[0])},required(...s){return nE(mw,this,s[0])}})});function Ue(e,t){const s={type:"object",shape:e??{},...De(t)};return new sA(s)}const iA=ge("ZodUnion",(e,t)=>{D2.init(e,t),Ut.init(e,t),e._zod.processJSONSchema=(s,a,o)=>tC(e,s,a,o),e.options=t.options});function aA(e,t){return new iA({type:"union",options:e,...De(t)})}const oA=ge("ZodIntersection",(e,t)=>{$2.init(e,t),Ut.init(e,t),e._zod.processJSONSchema=(s,a,o)=>nC(e,s,a,o)});function rA(e,t){return new oA({type:"intersection",left:e,right:t})}const Dv=ge("ZodRecord",(e,t)=>{R2.init(e,t),Ut.init(e,t),e._zod.processJSONSchema=(s,a,o)=>sC(e,s,a,o),e.keyType=t.keyType,e.valueType=t.valueType});function lA(e,t,s){return!t||!t._zod?new Dv({type:"record",keyType:ue(),valueType:e,...De(t)}):new Dv({type:"record",keyType:e,valueType:t,...De(s)})}const Xm=ge("ZodEnum",(e,t)=>{L2.init(e,t),Ut.init(e,t),e._zod.processJSONSchema=(a,o,l)=>FT(e,a,o),e.enum=t.entries,e.options=Object.values(t.entries);const s=new Set(Object.keys(t.entries));e.extract=(a,o)=>{const l={};for(const c of a)if(s.has(c))l[c]=t.entries[c];else throw new Error(`Key ${c} not found in enum`);return new Xm({...t,checks:[],...De(o),entries:l})},e.exclude=(a,o)=>{const l={...t.entries};for(const c of a)if(s.has(c))delete l[c];else throw new Error(`Key ${c} not found in enum`);return new Xm({...t,checks:[],...De(o),entries:l})}});function ao(e,t){const s=Array.isArray(e)?Object.fromEntries(e.map(a=>[a,a])):e;return new Xm({type:"enum",entries:s,...De(t)})}const cA=ge("ZodTransform",(e,t)=>{q2.init(e,t),Ut.init(e,t),e._zod.processJSONSchema=(s,a,o)=>QT(e,s),e._zod.parse=(s,a)=>{if(a.direction==="backward")throw new Bx(e.constructor.name);s.addIssue=l=>{if(typeof l=="string")s.issues.push(Ll(l,s.value,t));else{const c=l;c.fatal&&(c.continue=!1),c.code??(c.code="custom"),c.input??(c.input=s.value),c.inst??(c.inst=e),s.issues.push(Ll(c))}};const o=t.transform(s.value,s);return o instanceof Promise?o.then(l=>(s.value=l,s.fallback=!0,s)):(s.value=o,s.fallback=!0,s)}});function uA(e){return new cA({type:"transform",transform:e})}const pw=ge("ZodOptional",(e,t)=>{iw.init(e,t),Ut.init(e,t),e._zod.processJSONSchema=(s,a,o)=>uw(e,s,a,o),e.unwrap=()=>e._zod.def.innerType});function $v(e){return new pw({type:"optional",innerType:e})}const dA=ge("ZodExactOptional",(e,t)=>{U2.init(e,t),Ut.init(e,t),e._zod.processJSONSchema=(s,a,o)=>uw(e,s,a,o),e.unwrap=()=>e._zod.def.innerType});function fA(e){return new dA({type:"optional",innerType:e})}const pA=ge("ZodNullable",(e,t)=>{B2.init(e,t),Ut.init(e,t),e._zod.processJSONSchema=(s,a,o)=>iC(e,s,a,o),e.unwrap=()=>e._zod.def.innerType});function Rv(e){return new pA({type:"nullable",innerType:e})}const mA=ge("ZodDefault",(e,t)=>{H2.init(e,t),Ut.init(e,t),e._zod.processJSONSchema=(s,a,o)=>oC(e,s,a,o),e.unwrap=()=>e._zod.def.innerType,e.removeDefault=e.unwrap});function hA(e,t){return new mA({type:"default",innerType:e,get defaultValue(){return typeof t=="function"?t():Zx(t)}})}const gA=ge("ZodPrefault",(e,t)=>{I2.init(e,t),Ut.init(e,t),e._zod.processJSONSchema=(s,a,o)=>rC(e,s,a,o),e.unwrap=()=>e._zod.def.innerType});function yA(e,t){return new gA({type:"prefault",innerType:e,get defaultValue(){return typeof t=="function"?t():Zx(t)}})}const mw=ge("ZodNonOptional",(e,t)=>{Z2.init(e,t),Ut.init(e,t),e._zod.processJSONSchema=(s,a,o)=>aC(e,s,a,o),e.unwrap=()=>e._zod.def.innerType});function _A(e,t){return new mw({type:"nonoptional",innerType:e,...De(t)})}const bA=ge("ZodCatch",(e,t)=>{P2.init(e,t),Ut.init(e,t),e._zod.processJSONSchema=(s,a,o)=>lC(e,s,a,o),e.unwrap=()=>e._zod.def.innerType,e.removeCatch=e.unwrap});function vA(e,t){return new bA({type:"catch",innerType:e,catchValue:typeof t=="function"?t:()=>t})}const xA=ge("ZodPipe",(e,t)=>{Y2.init(e,t),Ut.init(e,t),e._zod.processJSONSchema=(s,a,o)=>cC(e,s,a,o),e.in=t.in,e.out=t.out});function Lv(e,t){return new xA({type:"pipe",in:e,out:t})}const wA=ge("ZodReadonly",(e,t)=>{V2.init(e,t),Ut.init(e,t),e._zod.processJSONSchema=(s,a,o)=>uC(e,s,a,o),e.unwrap=()=>e._zod.def.innerType});function SA(e){return new wA({type:"readonly",innerType:e})}const kA=ge("ZodLazy",(e,t)=>{K2.init(e,t),Ut.init(e,t),e._zod.processJSONSchema=(s,a,o)=>dC(e,s,a,o),e.unwrap=()=>e._zod.def.getter()});function jA(e){return new kA({type:"lazy",getter:e})}const NA=ge("ZodCustom",(e,t)=>{G2.init(e,t),Ut.init(e,t),e._zod.processJSONSchema=(s,a,o)=>JT(e,s)});function EA(e,t={}){return BT(NA,e,t)}function TA(e,t){return HT(e,t)}const id=Ue({id:ue(),workflow:ue(),status:ue(),tenant_id:ue().optional(),started_at:ue().optional(),finished_at:ue().nullable().optional(),run_dir:ue().nullable().optional(),error:ue().nullable().optional()}).loose(),Ml=id.extend({run:id.optional()}),zh=Ue({runs:dt(id)}).loose(),Dh=Ue({run_id:ue(),status:ue()}).loose(),hw=Ue({run:id,cancelled:sn()}).loose(),Od=Ue({run_id:ue(),seq:cn(),kind:ue(),payload:io(),ts:ue()}).loose(),$h=dt(Od),gw=Ue({id:ue(),tenant_id:ue(),status:ue(),user_input:ue(),workspace_root:ue(),profile:ue(),runtime_kind:ue(),model:ue(),skill_name:ue().nullable().optional(),started_at:ue().optional(),finished_at:ue().nullable().optional(),final_answer:ue().nullable().optional(),error:ue().nullable().optional()}).loose(),yw=Ue({session_id:ue(),seq:cn(),kind:ue(),payload:io(),ts:ue()}).loose(),CA=dt(yw),AA=dt(gw),_w=Ue({id:ue(),session_id:ue(),step_index:cn(),tool:ue(),source:ue().nullable().optional(),permissions:dt(ue()).optional(),idempotency:ue().optional(),params_summary:io().optional(),risk:ue(),reason:ue(),requested_at:ue(),expires_at:ue().nullable().optional()}).loose(),OA=dt(_w),MA=Ue({question:ue(),step_index:cn()}).loose(),zA=Ue({pending:MA.nullable()}).loose(),DA=ao(["ok","warning","fail"]),hm=Ue({path:ue(),source:ue(),exists:sn(),writable:sn(),error:ue().nullable().optional()}).loose(),$A=Ue({version:ue().optional(),profile:ue().optional(),status:DA,features:Ue({rag:sn().optional(),plugin:sn().optional(),mcp_workflow_nodes:sn().optional()}).loose().optional(),config:Ue({models_config_source:ue().optional(),models_config_path:ue().optional(),models_config_exists:sn().optional(),models_config_loadable:sn().optional(),models:cn().optional(),providers:cn().optional(),missing_env_vars:dt(ue()).optional(),warnings:dt(ue()).optional(),error:ue().nullable().optional()}).loose().optional(),security:Ue({env_var:ue().optional(),profile:ue().optional(),warning:ue().nullable().optional()}).loose().optional(),sandbox:Ue({backend:ue().optional(),enforcement:ue().optional(),enforcing:sn().optional(),capabilities:dt(ue()).optional(),warnings:dt(ue()).optional()}).loose().optional(),environment:Ue({yanshi_run_dir:ue().nullable().optional(),yanshi_trace_dir:ue().nullable().optional(),yanshi_api_token_set:sn().optional(),yanshi_skills_index:ue().nullable().optional()}).loose().optional(),disk:Ue({run_dir:hm.optional(),trace_dir:hm.optional(),marketplace_cache:hm.optional()}).loose().optional()}).loose(),RA=ao(["string","number","boolean","json"]),LA=Ue({name:ue(),description:ue().nullable().optional(),required:sn(),default:io().optional(),kind:RA}).loose(),bw=jA(()=>Ue({id:ue(),type:ue(),dependencies:dt(ue()),input_mapping:lA(ue(),ue()),run_if:ue().nullable().optional(),timeout_ms:cn().nullable().optional(),max_retries:cn().nullable().optional(),parameters:io(),sub_nodes:dt(bw).optional(),level:cn()}).loose()),qA=ao(["dependency","input_mapping","condition"]),UA=Ue({from:ue(),to:ue(),kind:qA,label:ue().nullable().optional()}).loose(),BA=Ue({valid:sn(),issues:dt(ue()),warnings:dt(ue())}).loose(),HA=Ue({valid:sn(),name:ue().nullable().optional(),cost_limit_usd:cn().nullable().optional(),inputs:dt(LA),nodes:dt(bw),edges:dt(UA),execution_order:dt(ue()),levels:dt(dt(ue())),diagnostics:BA}).loose(),IA=Ue({node_id:ue(),phase:ao(["before_node","after_node"]),next_nodes:dt(ue())}).loose(),ZA=Ue({step_every_node:sn(),breakpoints:dt(ue()),paused_at:IA.nullable().optional()}).loose(),qv=Ue({run_id:ue(),status:ue(),active:sn(),snapshot:ZA.nullable().optional(),resumed:sn().nullable().optional()}).loose(),PA=Ue({id:ue(),node_id:ue(),name:ue(),mime_type:ue().nullable().optional(),size_bytes:cn().nullable().optional(),sha256:ue().nullable().optional(),created_at:ue().optional()}).loose(),YA=Ue({run_id:ue(),artifacts:dt(PA)}).loose(),VA=Ue({node_id:ue(),tool_call_id:ue(),tool:ue(),step_index:cn(),idempotency:ao(["idempotent","non_idempotent","unknown"]),decision:ao(["skip","replay","requires_manual"]),reason:ue(),has_recorded_result:sn()}).loose(),KA=Ue({total:cn(),to_replay:cn(),to_skip:cn(),requires_manual:cn()}).loose(),GA=Ue({schema_version:cn(),workflow_id:ue(),last_completed_node:ue(),status:ue(),created_at:ue(),tool_calls:dt(VA),summary:KA,force_replay:sn()}).loose(),XA=Ue({approvals:dt(_w)}).loose(),FA=Ue({run_id:ue(),request_id:ue(),resolved:sn()}).loose(),JA=Ue({question_id:ue(),node_id:ue(),question:ue(),created_at:ue()}).loose(),QA=Ue({pending:dt(JA)}).loose(),WA=Ue({run_id:ue(),question_id:ue(),resolved:sn()}).loose(),Rh=Ue({id:ue(),tenant_id:ue(),title:ue(),created_at:ue(),updated_at:ue()}).loose(),eO=Ue({notebooks:dt(Rh)}).loose(),vw=Ue({id:ue(),notebook_id:ue(),tenant_id:ue(),kind:ue(),title:ue(),uri:ue().nullable().optional(),status:ue(),error:ue().nullable().optional(),sha256:ue().nullable().optional(),created_at:ue(),updated_at:ue()}).loose(),tO=Ue({notebook:Rh,sources:dt(vw)}).loose(),nO=Ue({source_id:ue(),chunk_id:ue(),content:ue(),score:cn()}).loose(),sO=Ue({chunks:dt(nO)}).loose(),iO=Ue({run_id:ue(),kind:ue(),status:ue().optional()}).loose(),aO=Ue({kind:ue(),id:ue(),run_id:ue(),node_id:ue(),name:ue(),path_or_url:ue(),mime_type:ue().nullable().optional(),created_at:ue(),tenant_id:ue().optional(),size_bytes:cn().nullable().optional(),sha256:ue().nullable().optional()}).loose(),oO=Ue({notebook_id:ue(),artifacts:dt(aO)}).loose();async function tt(e,t,s){const a=await t.json();return xw(e,a,s)}function xw(e,t,s){const a=e.safeParse(t);if(!a.success)throw new rO(s,t,a.error);return a.data}class rO extends Error{contextLabel;raw;zodError;constructor(t,s,a){const o=a.issues.slice(0,3).map(l=>`${l.path.join(".")||"<root>"}: ${l.message}`).join("; ");super(`${t}: response did not match expected shape (${a.issues.length} issue${a.issues.length===1?"":"s"}: ${o})`),this.name="SchemaValidationError",this.contextLabel=t,this.raw=s,this.zodError=a}}const Uv="yanshi.ui.apiToken",Fm="yanshi.ui.workflowDraft",Ni="yanshi.ui.tenantId",Lh="yanshi.ui.apiBaseUrl",Bv="yanshi.ui.studioMode",gm="yanshi.ui.studioPreset",ad="yanshi.ui.studioModelOverride",Eu="yanshi.ui.studioPersonalTemplates",ym="yanshi.ui.run.eventFilter.",Hv="yanshi.ui.newForm.workflow",Iv="yanshi.ui.newForm.tenant",Zv="yanshi.ui.newForm.profile",Pv="yanshi.ui.newForm.inputs",bt=(e,t)=>{try{return window.localStorage.getItem(e)??t}catch{return t}},vt=(e,t)=>{try{window.localStorage.setItem(e,t)}catch{}},lO=(e,t)=>{try{return window.sessionStorage.getItem(e)??t}catch{return t}},cO=(e,t)=>{try{t?window.sessionStorage.setItem(e,t):window.sessionStorage.removeItem(e)}catch{}},Jm=e=>e.trim().replace(/\/+$/,""),ww=()=>{const e=typeof window<"u"?Jm(bt(Lh,"")):"";return e||(typeof window<"u"&&typeof window.__YANSHI_API_BASE__=="string"?Jm(window.__YANSHI_API_BASE__):"")},uO=e=>{vt(Lh,Jm(e))},Yv=()=>typeof window<"u"&&(!!window.__TAURI_INTERNALS__||!!window.__YANSHI_API_BASE__),tc=e=>{if(/^https?:\/\//i.test(e))return e;const t=ww();return t?e.startsWith("/")?`${t}${e}`:`${t}/${e}`:e},Me=(e,t,s={},a)=>{const o=new Headers(s.headers),l=t.trim();l&&o.set("Authorization",`Bearer ${l}`);const c=a?.trim();return c&&o.set("X-Yanshi-Tenant",c),fetch(tc(e),{...s,headers:o})},ts=e=>{const t=e instanceof Error?e.message:String(e);return/failed to fetch|networkerror|load failed|econnrefused/i.test(t)?`无法连接后端网关（${t}）。请先启动 \`yanshi serve\`，或在 Studio 顶栏「网关配置」中填写服务地址（Tauri 桌面端会自动启动内置引擎）。`:t},Md=(e,t,s)=>{const a=[];let o=e,l=o.indexOf(`

`);for(;l>=0;){const c=o.slice(0,l);o=o.slice(l+2);const f=c.split(`
`).filter(m=>m.startsWith("data:")).map(m=>m.slice(5).trimStart()).join(`
`);f&&a.push(xw(t,JSON.parse(f),s)),l=o.indexOf(`

`)}return{events:a,rest:o}},dO={"yanshi.ui.tenantId":"ui.run-console.tenant","yanshi.ui.newForm.tenant":"ui.new-form.tenant","yanshi.ui.newForm.profile":"ui.new-form.profile","yanshi.ui.harness.newForm.tenant_id":"ui.harness-new-form.tenant","yanshi.ui.harness.newForm.profile":"ui.harness-new-form.profile","yanshi.ui.harness.newForm.runtime_kind":"ui.harness-new-form.runtime"},Vv="yanshi.ui.run.eventFilter.",fO="ui.event-filter.";function pO(e){const t=dO[e];if(t)return t;if(e.startsWith(Vv)){const s=e.slice(Vv.length);return s?`${fO}${s}`:null}return null}function Sw(e){return{"X-Yanshi-Tenant":e}}async function mO(e,t){const s=await e("/v1/preferences",{headers:Sw(t)});if(!s.ok)throw new Error(`GET /v1/preferences failed: ${s.status} ${s.statusText}`);return(await s.json()).preferences??{}}async function hO(e,t,s){const a={preferences:s},o=await e("/v1/preferences",{method:"PUT",headers:{...Sw(t),"Content-Type":"application/json"},body:JSON.stringify(a)});if(!o.ok)throw new Error(`PUT /v1/preferences failed: ${o.status} ${o.statusText}`)}class gO{constructor(t,s){this.waitMs=t,this.onFlush=s}waitMs;onFlush;pending={};timer=null;enqueue(t,s){this.pending[t]=s,this.timer!==null&&clearTimeout(this.timer),this.timer=setTimeout(()=>this.flushNow(),this.waitMs)}flushNow(){if(this.timer!==null&&(clearTimeout(this.timer),this.timer=null),Object.keys(this.pending).length===0)return;const t=this.pending;this.pending={},this.onFlush(t)}cancel(){this.timer!==null&&(clearTimeout(this.timer),this.timer=null),this.pending={}}}const yO=500;function Kv(e){return(t,s={})=>Me(t,e,s)}function _O(e,t){const[s,a]=C.useState(null);C.useEffect(()=>{if(!t.trim()){a(null);return}let m=!1;return mO(Kv(e),t.trim()).then(p=>{m||a(p)}).catch(p=>{console.warn("preferences GET failed:",p),m||a(null)}),()=>{m=!0}},[e,t]);const o=C.useRef(e),l=C.useRef(t);o.current=e,l.current=t;const c=C.useRef(null);c.current===null&&(c.current=new gO(yO,m=>{const p=o.current,_=l.current.trim();_&&hO(Kv(p),_,m).catch(y=>{console.warn("preferences PUT failed:",y)})})),C.useEffect(()=>{const m=c.current;return()=>{m?.flushNow()}},[]);const f=C.useCallback((m,p)=>{const _=pO(m);_&&c.current?.enqueue(_,p)},[]);return{serverPrefs:s,syncToServer:f}}class bO extends kh.Component{state={error:null,componentStack:null};static getDerivedStateFromError(t){return{error:t}}componentDidCatch(t,s){console.error("[ErrorBoundary] caught error:",t,s),this.setState({componentStack:s.componentStack??null})}reset=()=>{this.setState({error:null,componentStack:null})};reload=()=>{window.location.reload()};render(){if(this.state.error===null)return this.props.children;const t=this.props.label??"Yanshi UI",s=this.state.error,a=this.state.componentStack??"(component stack not available)";return d.jsx("main",{className:"error-boundary",children:d.jsxs("section",{className:"error-boundary__panel",role:"alert",children:[d.jsx("p",{className:"error-boundary__kicker",children:"Yanshi"}),d.jsxs("h1",{children:[t," ran into an unhandled error"]}),d.jsx("p",{className:"error-boundary__summary",children:s.message||String(s)}),d.jsxs("details",{className:"error-boundary__details",open:!0,children:[d.jsx("summary",{children:"Component stack"}),d.jsx("pre",{children:a.trim()})]}),d.jsxs("p",{className:"error-boundary__hint",children:["The full stack and React DevTools breadcrumb were written to the browser console. Use ",d.jsx("strong",{children:"Reset"})," to re-mount the tree without a full page reload, or ",d.jsx("strong",{children:"Reload"})," if state is unrecoverable."]}),d.jsxs("div",{className:"error-boundary__actions",children:[d.jsx("button",{type:"button",onClick:this.reset,children:"Reset"}),d.jsx("button",{type:"button",onClick:this.reload,children:"Reload page"})]})]})})}}const Qm=new Set;let Tu=null,Wm={pathname:"/ui/",search:""};const qh=e=>{const t=e.trim();return!t||t==="/"||t==="/ui"?"/ui/":t.startsWith("?")?`/ui/${t}`:t.startsWith("/")?t:`/${t}`},kw=e=>{const t=qh(e),s=t.indexOf("#"),a=s>=0?t.slice(0,s):t,o=a.indexOf("?");return o<0?{pathname:a,search:""}:{pathname:a.slice(0,o)||"/ui/",search:a.slice(o)}},jw=()=>{if(Tu!==null)return Tu;if(typeof window>"u")return"embedded";const e=window.location.pathname;return Tu=e==="/ui"||e.startsWith("/ui/")?"history":"embedded",Tu},od=()=>{if(typeof window>"u")return Wm;if(jw()==="history")return{pathname:window.location.pathname,search:window.location.search};const e=window.location.hash;return e.startsWith("#/")?kw(e.slice(1)):Wm},vO=()=>{for(const e of Qm)e()},xO=e=>(Qm.add(e),()=>{Qm.delete(e)}),gs=(e,t={})=>{const s=qh(e);if(Wm=kw(s),typeof window<"u")if(jw()==="history")t.replace?window.history.replaceState(null,"",s):window.history.pushState(null,"",s);else{const o=`#${s}`;try{t.replace?window.history.replaceState(null,"",o):window.history.pushState(null,"",o)}catch{}}vO()},wO=(e,t)=>!(e.defaultPrevented||e.button!==0||e.metaKey||e.ctrlKey||e.shiftKey||e.altKey||t&&t!=="_self"),Uh=()=>{const[e,t]=C.useState(()=>od());return C.useEffect(()=>{const s=()=>{const o=od();t(l=>l.pathname===o.pathname&&l.search===o.search?l:o)},a=xO(s);return window.addEventListener("popstate",s),window.addEventListener("hashchange",s),()=>{a(),window.removeEventListener("popstate",s),window.removeEventListener("hashchange",s)}},[]),e};function ln({to:e,replace:t=!1,onClick:s,target:a,children:o,...l}){const c=qh(e),f=m=>{s?.(m),wO(m,a)&&(m.preventDefault(),gs(c,{replace:t}))};return d.jsx("a",{href:c,target:a,onClick:f,...l,children:o})}const SO=[{id:"notebooks",label:"内容笔记本",to:"/ui/notebooks"},{id:"console",label:"经典事件台",to:"/ui/console"},{id:"runs-new",label:"新建运行",to:"/ui/runs/new"},{id:"harness",label:"Harness 会话",to:"/ui/harness/sessions"},{id:"diagnostics",label:"系统诊断",to:"/ui/diagnostics"}];function _a({eyebrow:e,title:t,subtitle:s,activeSection:a,actions:o}){return d.jsxs("header",{className:"studio-subpage-header",children:[d.jsxs("div",{className:"studio-subpage-header__lead",children:[d.jsx(ln,{to:"/ui/",className:"studio-subpage-back","data-testid":"subpage-back-studio",title:"返回 Yanshi Studio 可视化工作台",children:"← Studio 工作台"}),d.jsxs("div",{className:"studio-subpage-header__titles",children:[d.jsx("p",{className:"eyebrow",children:e}),d.jsx("h1",{children:t}),s?d.jsx("p",{className:"studio-subpage-subtitle",children:s}):null]})]}),d.jsxs("div",{className:"studio-subpage-header__right",children:[d.jsx("nav",{className:"studio-subpage-nav","aria-label":"控制台全局导航",children:SO.map(l=>{const c=a===l.id;return d.jsx(ln,{to:l.to,className:`studio-subpage-nav__link ${c?"studio-subpage-nav__link--active":""}`,"aria-current":c?"page":void 0,children:l.label},l.id)})}),o?d.jsx("div",{className:"studio-subpage-header__actions",children:o}):null]})]})}const eh=new Set(["name","description","version","inputs","nodes","id","type","dependencies","depends_on","run_if","condition","input_mapping","parameters","retry","timeout_secs","output_format","sub_nodes"]),kO=new Set(["template","llm","agent","skill_agent","map","while","conditional","render_slides","render_video","render_document","tts","image_gen","video_gen","http","web_search","repo_snapshot","pdf_extract","video_ingest","asr","figure","render_table","render_mindmap","render_markdown","text_to_image","text_to_video","file","shell","batch","markmap","arxiv","mcp_tool","rag_retrieve","rag_ingest","code_exec"]),_m=/(\{\{[\s\S]*?\}\}|\{%[\s\S]*?%\}|\{#[\s\S]*?#\})/g,jO=/^(?:true|false|null|~|True|False|NULL)$/,NO=/^[+-]?(?:0x[0-9a-fA-F]+|\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)$/,EO=/^([|>](?:[-+]\d*|\d*[-+]?))(\s*(?:#.*)?)$/;function Ie(e,t,s){if(!s)return;const a=e[e.length-1];a&&a.kind===t&&t!=="template"?a.text+=s:e.push({kind:t,text:s})}function Zu(e,t,s){if(!s)return;if(!s.includes("{")){Ie(e,t,s);return}let a=0;_m.lastIndex=0;let o=_m.exec(s);for(;o!==null;)o.index>a&&Ie(e,t,s.slice(a,o.index)),Ie(e,"template",o[0]),a=o.index+o[0].length,o=_m.exec(s);a<s.length&&Ie(e,t,s.slice(a))}function TO(e,t){return jO.test(e)?"boolean":NO.test(e)?"number":t==="type"&&kO.has(e)?"node-type":"string"}function Gv(e,t,s){const a=t.trimStart(),o=t.length-a.length;if(o>0&&Ie(e,"plain",t.slice(0,o)),!a)return{enteredBlockScalar:!1};const l=EO.exec(a);if(l){Ie(e,"punctuation",l[1]);const _=l[2];if(_){const y=_.indexOf("#");y>=0?(Ie(e,"plain",_.slice(0,y)),Ie(e,"comment",_.slice(y))):Ie(e,"plain",_)}return{enteredBlockScalar:!0}}let c=0;const f=a.length;let m=s,p=0;for(;c<f;){const _=a[c];if(_===" "||_==="	"){let w=c+1;for(;w<f&&(a[w]===" "||a[w]==="	");)w+=1;Ie(e,"plain",a.slice(c,w)),c=w;continue}if(_==="#"&&(c===0||a[c-1]===" "||a[c-1]==="	")){Ie(e,"comment",a.slice(c));break}if(_==="{"&&c+1<f&&(a[c+1]==="{"||a[c+1]==="%"||a[c+1]==="#")){const w=a[c+1]==="{"?"}}":a[c+1]==="%"?"%}":"#}",k=a.indexOf(w,c+2);if(k>=0){Ie(e,"template",a.slice(c,k+2)),c=k+2;continue}}if(_==="["||_==="]"||_==="{"||_==="}"||_===","){(_==="{"||_==="[")&&(p+=1),(_==="}"||_==="]")&&p>0&&(p-=1),(_===","||_==="{")&&(m=null),Ie(e,"punctuation",_),c+=1;continue}if(_==='"'){let w=c+1;for(;w<f;){if(a[w]==="\\"){w=Math.min(f,w+2);continue}if(a[w]==='"'){w+=1;break}w+=1}const k=a.slice(c,w),j=a.slice(w),A=/^(\s*):(?=\s|[,}\]]|$)/.exec(j);if(p>0&&A){const N=k.slice(1,k.endsWith('"')?-1:k.length);Ie(e,eh.has(N)?"schema-key":"key",k),Ie(e,"plain",A[1]),Ie(e,"punctuation",":"),m=N,c=w+A[1].length+1}else Zu(e,"string",k),c=w;continue}if(_==="'"){let w=c+1;for(;w<f;){if(a[w]==="'"&&a[w+1]==="'"){w+=2;continue}if(a[w]==="'"){w+=1;break}w+=1}Zu(e,"string",a.slice(c,w)),c=w;continue}if(_==="&"||_==="*"||_==="!"){let w=c+1;for(;w<f&&a[w]!==" "&&a[w]!=="	"&&a[w]!==","&&a[w]!=="]"&&a[w]!=="}";)w+=1;Ie(e,"anchor",a.slice(c,w)),c=w;continue}if(p>0){const w=/^([A-Za-z0-9_.-]+)(\s*):(?=\s|[,}\]]|$)/.exec(a.slice(c));if(w){const k=w[1];Ie(e,eh.has(k)?"schema-key":"key",k),Ie(e,"plain",w[2]),Ie(e,"punctuation",":"),m=k,c+=w[1].length+w[2].length+1;continue}}let y=c;for(;y<f;){const w=a[y];if(w==="#"&&(a[y-1]===" "||a[y-1]==="	")||p>0&&(w===","||w==="]"||w==="}"))break;y+=1}let b=y;for(;b>c&&(a[b-1]===" "||a[b-1]==="	");)b-=1;const v=a.slice(c,b),S=TO(v,m);Zu(e,S,v),y>b&&Ie(e,"plain",a.slice(b,y)),m=null,c=y}return{enteredBlockScalar:!1}}function CO(e){const t=e.split(`
`),s=[];let a=null,o=null;for(const l of t){const c=l.endsWith("\r"),f=c?l.slice(0,-1):l,m=[];let p=0;for(;p<f.length&&f[p]===" ";)p+=1;const _=f.slice(p);if(a!==null){if(_.trim().length===0){Ie(m,"plain",f),c&&Ie(m,"plain","\r"),s.push(m);continue}if(o===null&&p>a&&(o=p),o!==null&&p>=o){Ie(m,"plain",f.slice(0,p)),Zu(m,"string",_),c&&Ie(m,"plain","\r"),s.push(m);continue}a=null,o=null}if(_.length===0){Ie(m,"plain",f),c&&Ie(m,"plain","\r"),s.push(m);continue}if(p===0&&(/^(?:---|\.\.\.)(?=\s|$)/.test(f)||f.startsWith("%YAML")||f.startsWith("%TAG"))){const j=f.indexOf(" #");j>=0?(Ie(m,"directive",f.slice(0,j)),Ie(m,"plain"," "),Ie(m,"comment",f.slice(j+1))):Ie(m,"directive",f),c&&Ie(m,"plain","\r"),s.push(m);continue}if(p>0&&Ie(m,"plain",f.slice(0,p)),_.startsWith("#")){Ie(m,"comment",_),c&&Ie(m,"plain","\r"),s.push(m);continue}let y=p;for(;y<f.length&&f[y]==="-"&&(y+1===f.length||f[y+1]===" "||f[y+1]==="	");){Ie(m,"punctuation","-"),y+=1;let j=y;for(;j<f.length&&(f[j]===" "||f[j]==="	");)j+=1;j>y&&(Ie(m,"plain",f.slice(y,j)),y=j)}const b=p,v=f.slice(y);if(v.length===0){c&&Ie(m,"plain","\r"),s.push(m);continue}if(v.startsWith("#")){Ie(m,"comment",v),c&&Ie(m,"plain","\r"),s.push(m);continue}const S=/^([A-Za-z0-9_.-]+)(\s*):(?=\s|$)/.exec(v),w=S?null:/^("(?:\\.|[^"\\])*"|'(?:''|[^'])*')(\s*):(?=\s|$)/.exec(v),k=S??w;if(k){const j=k[1],A=j.startsWith('"')||j.startsWith("'")?j.slice(1,-1):j;Ie(m,eh.has(A)?"schema-key":"key",j),Ie(m,"plain",k[2]),Ie(m,"punctuation",":");const N=v.slice(j.length+k[2].length+1),{enteredBlockScalar:T}=Gv(m,N,A);T&&(a=b,o=null)}else{const{enteredBlockScalar:j}=Gv(m,v,null);j&&(a=b,o=null)}c&&Ie(m,"plain","\r"),s.push(m)}return s}function AO(e,t,s,a,o){if(a==="Tab"){const l=t!==s&&e.slice(t,s).includes(`
`);if(!o&&!l)return{value:e.slice(0,t)+"  "+e.slice(s),selectionStart:t+2,selectionEnd:t+2};const c=e.lastIndexOf(`
`,Math.max(0,t-1))+1,f=s>t&&e[s-1]===`
`?s-1:s,m=e.indexOf(`
`,f),p=m===-1?e.length:m,y=e.slice(c,p).split(`
`);let b=0,v=0;const S=y.map((A,N)=>{if(!o)return N===0&&(b=2),v+=2,"  "+A;const T=A.startsWith("  ")?2:A.startsWith(" ")?1:0;return N===0&&(b=-T),v-=T,A.slice(T)}),w=e.slice(0,c)+S.join(`
`)+e.slice(p),k=Math.max(c,t+b),j=t===s?k:Math.max(k,s+v);return{value:w,selectionStart:k,selectionEnd:j}}if(a==="Enter"&&!o){const l=e.lastIndexOf(`
`,Math.max(0,t-1))+1,c=e.slice(l,t),f=/^( *)/.exec(c),m=f?f[1]:"",_=c.replace(/#.*$/,"").trimEnd().endsWith(":")?"  ":"",y=`
`+m+_,b=e.slice(0,t)+y+e.slice(s),v=t+y.length;return{value:b,selectionStart:v,selectionEnd:v}}return null}function OO(e){switch(e.kind){case"plain":return;case"schema-key":return"yaml-tok yaml-tok--schema-key";case"key":return"yaml-tok yaml-tok--key";case"node-type":return"yaml-tok yaml-tok--node-type";case"string":return"yaml-tok yaml-tok--string";case"template":return e.text.startsWith("{%")?"yaml-tok yaml-tok--template-ctrl":e.text.startsWith("{#")?"yaml-tok yaml-tok--comment":"yaml-tok yaml-tok--template";case"number":return"yaml-tok yaml-tok--number";case"boolean":return"yaml-tok yaml-tok--boolean";case"comment":return"yaml-tok yaml-tok--comment";case"punctuation":return"yaml-tok yaml-tok--punct";case"directive":return"yaml-tok yaml-tok--directive";case"anchor":return"yaml-tok yaml-tok--anchor";default:return}}function rd({value:e,onChange:t,testId:s,id:a,ariaLabel:o="YAML 编辑器",minHeight:l=260,rows:c,readOnly:f=!1,className:m="",onExpandToggle:p,isExpanded:_=!1}){const y=C.useRef(null),b=C.useRef(null),v=C.useRef(null),[S,w]=C.useState({line:1,col:1}),k=C.useMemo(()=>CO(e),[e]),j=C.useCallback(()=>{const E=y.current,O=b.current,R=v.current;E&&(O&&(E.clientWidth>0&&(O.style.width=`${E.clientWidth}px`),E.clientHeight>0&&(O.style.height=`${E.clientHeight}px`),O.scrollTop=E.scrollTop,O.scrollLeft=E.scrollLeft),R&&(E.clientHeight>0&&(R.style.height=`${E.clientHeight}px`),R.scrollTop=E.scrollTop))},[]),A=C.useCallback(()=>{const E=y.current;if(!E)return;const O=E.selectionStart??0,R=E.value.slice(0,O),q=R.lastIndexOf(`
`),Z=q===-1?1:R.split(`
`).length,oe=O-q;w(Q=>Q.line===Z&&Q.col===oe?Q:{line:Z,col:oe})},[]);C.useEffect(()=>{j()},[e,j]),C.useEffect(()=>{const E=y.current;if(!E||typeof ResizeObserver>"u")return;const O=new ResizeObserver(()=>{j()});return O.observe(E),()=>O.disconnect()},[j]);const N=E=>{if(f||E.nativeEvent.isComposing||E.key!=="Tab"&&E.key!=="Enter"||E.metaKey||E.ctrlKey||E.altKey)return;const O=E.currentTarget,R=AO(e,O.selectionStart,O.selectionEnd,E.key,E.shiftKey);R&&(E.preventDefault(),t(R.value),requestAnimationFrame(()=>{const q=y.current;q&&(q.setSelectionRange(R.selectionStart,R.selectionEnd),j(),A())}))},T=c?Math.max(160,c*20+24):l;return d.jsxs("div",{className:`yaml-code-editor ${_?"yaml-code-editor--expanded":""} ${m}`.trim(),children:[d.jsxs("div",{className:"yaml-code-editor__body",style:_?void 0:{minHeight:`${T}px`},children:[d.jsx("div",{ref:v,className:"yaml-code-editor__gutter","aria-hidden":"true",children:k.map((E,O)=>{const R=O+1;return d.jsx("span",{className:S.line===R?"yaml-code-editor__ln active":"yaml-code-editor__ln",children:R},R)})}),d.jsxs("div",{className:"yaml-code-editor__stage",children:[d.jsx("pre",{ref:b,className:"yaml-code-editor__backdrop","aria-hidden":"true",children:k.map((E,O)=>{const R=O===k.length-1;return d.jsxs(kh.Fragment,{children:[E.map((q,Z)=>{const oe=OO(q);return oe?d.jsx("span",{className:oe,children:q.text},Z):q.text}),R?E.length===0?" ":"":`
`]},O)})}),d.jsx("textarea",{ref:y,id:a,"data-testid":s,"aria-label":o,className:"yaml-code-editor__input",value:e,readOnly:f,rows:c,spellCheck:!1,autoCapitalize:"off",autoComplete:"off",autoCorrect:"off",wrap:"off",onChange:E=>{t(E.target.value),requestAnimationFrame(()=>{j(),A()})},onScroll:j,onKeyDown:N,onKeyUp:A,onClick:A,onSelect:A})]})]}),d.jsxs("div",{className:"yaml-code-editor__footer",children:[d.jsxs("span",{className:"yaml-code-editor__meta",children:["YAML · 2 Spaces · Ln ",S.line,", Col ",S.col," · 共 ",k.length," 行"]}),p&&d.jsx("button",{type:"button",className:"yaml-code-editor__expand-btn",onClick:p,children:_?"⤡ 退出放大":"⤢ 放大编辑"})]})]})}const MO=e=>{const t=e.trim();return t?t.length<=4?`••${t}`:`••••${t.slice(-4)}`:""},zO=e=>{const t=[],s=e.config;if(s){const f=s.missing_env_vars??[],m=s.error||f.length>0?"warning":"ok",p=s.error?s.error:f.length>0?`Missing env vars: ${f.join(", ")}`:`${s.models??0} models / ${s.providers??0} providers`;t.push({component:"Models config",status:m,detail:p})}const a=e.security;a&&t.push({component:"Security profile",status:a.warning?"warning":"ok",detail:a.warning??`profile: ${a.profile??"unknown"}`});const o=e.sandbox;if(o){const f=o.enforcing?"ok":((o.warnings?.length??0)>0,"warning"),m=`backend: ${o.backend??"unknown"} (${o.enforcement??"unknown"})`;t.push({component:"OS sandbox",status:f,detail:m})}const l=[["Run dir",e.disk?.run_dir],["Trace dir",e.disk?.trace_dir],["Marketplace cache",e.disk?.marketplace_cache]];for(const[f,m]of l){if(!m)continue;const p=m.exists?m.writable?"ok":"fail":"warning",_=`${m.path} (${m.source}${m.writable?", writable":""})`;t.push({component:f,status:p,detail:_})}const c=e.environment;return c&&t.push({component:"YANSHI_API_TOKEN",status:c.yanshi_api_token_set?"ok":"warning",detail:c.yanshi_api_token_set?"set (value masked)":"not set"}),t};function DO({apiToken:e,onTokenChange:t}){const[s,a]=C.useState(null),[o,l]=C.useState(!1),[c,f]=C.useState(null),m=async()=>{l(!0),f(null);try{const v=await Me("/v1/diagnostics",e);if(!v.ok){f(`HTTP ${v.status}`),a(null);return}const S=await tt($A,v,"GET /v1/doctor");a(S)}catch(v){f(ts(v)),a(null)}finally{l(!1)}};C.useEffect(()=>{m()},[]);const p=C.useMemo(()=>s?zO(s):[],[s]),_=v=>{const S=v==="ok"?"PASS":v==="warning"?"WARN":"FAIL";return d.jsx("span",{className:`diag-badge diag-badge-${v}`,children:S})},y=s?.status,b=e?MO(e):"(none)";return d.jsxs("main",{className:"shell diagnostics-shell",children:[d.jsx(_a,{eyebrow:"Yanshi / System",title:"Diagnostics",subtitle:"In-process model registry, security profile, OS sandbox, and storage health check.",activeSection:"diagnostics"}),d.jsxs("div",{className:"diagnostics-panel",children:[d.jsx("header",{className:"diagnostics-header",children:d.jsxs("div",{className:"diagnostics-controls",children:[d.jsxs("label",{children:["API token (last 4 shown):"," ",d.jsx("code",{className:"diag-token",children:b})]}),d.jsx("input",{type:"password",placeholder:"paste bearer token",value:e,onChange:v=>t(v.target.value)}),d.jsx("button",{type:"button",onClick:()=>{m()},disabled:o,children:o?"Refreshing…":"Refresh"})]})}),c&&d.jsxs("div",{className:"diagnostics-error",children:[d.jsx("strong",{children:"Error:"})," ",c]}),y&&d.jsxs("div",{className:`diagnostics-overall diagnostics-overall-${y}`,children:["Overall: ",_(y),s?.version&&d.jsxs("span",{className:"diag-version",children:["version ",s.version]}),s?.profile&&d.jsxs("span",{className:"diag-profile",children:["profile ",s.profile]})]}),d.jsxs("table",{className:"diagnostics-table",children:[d.jsx("thead",{children:d.jsxs("tr",{children:[d.jsx("th",{children:"Component"}),d.jsx("th",{children:"Status"}),d.jsx("th",{children:"Detail"})]})}),d.jsxs("tbody",{children:[p.length===0&&!c&&d.jsx("tr",{children:d.jsx("td",{colSpan:3,className:"diagnostics-empty",children:o?"Loading…":"No data yet"})}),p.map(v=>d.jsxs("tr",{children:[d.jsx("td",{children:v.component}),d.jsx("td",{children:_(v.status)}),d.jsx("td",{children:v.detail})]},v.component))]})]})]})]})}const Xv="yanshi.ui.harness.newForm.user_input",Fv="yanshi.ui.harness.newForm.workspace_root",Jv="yanshi.ui.harness.newForm.profile",Qv="yanshi.ui.harness.newForm.runtime_kind",Wv="yanshi.ui.harness.newForm.model",e0="yanshi.ui.harness.newForm.skill_name",ld="yanshi.ui.harness.newForm.tenant_id",Nw=e=>{switch(e){case"running":return"pending";case"awaiting_input":return"pending";case"completed":return"success";case"failed":return"danger";case"cancelled":return"danger";default:return"neutral"}},$O=e=>e?["completed","failed","cancelled"].includes(e.status):!0,RO=e=>{const s=(e??od().pathname).match(/^\/ui\/harness\/sessions\/([^/]+)\/?$/);if(!s)return null;const a=s[1];return a==="new"||a.length===0?null:decodeURIComponent(a)},t0=["dev","local","production"],n0=["react","plan_execute"],LO=e=>{if(e&&typeof e=="object"&&"delta"in e){const t=e.delta;return typeof t=="string"?t:null}return null},Zn=e=>e?new Intl.DateTimeFormat(void 0,{hour:"2-digit",minute:"2-digit",second:"2-digit"}).format(new Date(e)):"pending",dr=e=>e.run??e,zl=e=>{const t=e.toLowerCase();return t.includes("fail")||t.includes("error")||t.includes("denied")?"danger":t.includes("tool")||t.includes("policy")||t.includes("capability")?"tool":t.includes("agent")||t.includes("reflect")||t.includes("plan")||t.includes("step")?"agent":t.includes("complete")||t.includes("succeed")?"success":"neutral"},As=e=>JSON.stringify(e,null,2),cd=e=>e?["succeeded","failed","cancelled"].includes(e.status):!0,qO=(e,t)=>{for(let s=e.length-1;s>=0;s-=1)if(t(e[s]))return e[s]},bm=e=>{const t=e.payload;return String(t.node_id??t.node_name??t.node??t.step??"").trim()},Ew=e=>{const t=Math.max(0,Math.floor(e)),s=250*Math.pow(2,t);return Math.min(s,3e4)},UO=e=>{const t=Array.isArray(e)?e:e!==null&&typeof e=="object"&&"events"in e?e.events??[]:[];return $h.parse(t)},eo=(e,t)=>{const s=e.trim(),a=`/ui/runs/${encodeURIComponent(s)}/compare`,o=t?.trim()??"";if(o&&o!==s){const l=new URLSearchParams({against:o});return`${a}?${l.toString()}`}return a},BO=()=>{try{const t=new URLSearchParams(od().search).get("tenant");if(t&&t.trim())return t.trim()}catch{}return"default"};function HO({sessionId:e,apiToken:t,onTokenChange:s}){const[a]=C.useState(()=>BO()),[o,l]=C.useState(null),[c,f]=C.useState([]),[m,p]=C.useState(""),[_,y]=C.useState([]),[b,v]=C.useState(null),[S,w]=C.useState(!1),[k,j]=C.useState(null),[A,N]=C.useState(null),[T,E]=C.useState(null),[O,R]=C.useState("idle"),[q,Z]=C.useState(!1),[oe,Q]=C.useState(""),[ee,X]=C.useState("rerun"),L=F=>{if(F.kind==="token_delta"){const G=LO(F.payload);G&&p(P=>P+G);return}F.kind==="step_started"&&p(""),f(G=>G.some(P=>P.seq===F.seq)?G:[...G,F].sort((P,ae)=>P.seq-ae.seq))},I=F=>{f([...F].sort((G,P)=>G.seq-P.seq)),p("")},D=async()=>{try{const F=await Me(`/v1/harness/sessions/${e}`,t,{},a);if(!F.ok){const P=await F.text();throw new Error(`session fetch failed: HTTP ${F.status} ${P}`)}const G=await tt(gw,F,`GET /v1/harness/sessions/${e}`);l(G),N(null)}catch(F){N(F instanceof Error?F.message:String(F))}},U=async()=>{try{const F=await Me(`/v1/harness/sessions/${e}/events/history`,t,{},a);if(!F.ok){const P=await F.text();throw new Error(`events fetch failed: HTTP ${F.status} ${P}`)}const G=await tt(CA,F,`GET /v1/harness/sessions/${e}/events/history`);I(G)}catch(F){N(F instanceof Error?F.message:String(F))}},B=async()=>{try{const F=await Me(`/v1/harness/sessions/${e}/approvals`,t,{},a);if(!F.ok){const ae=await F.text();throw new Error(`approvals fetch failed: HTTP ${F.status} ${ae}`)}const G=await F.json(),P=OA.parse(G?.approvals??[]);y(P)}catch(F){N(F instanceof Error?F.message:String(F))}},V=async()=>{try{const F=await Me(`/v1/harness/sessions/${e}/interrupt`,t,{},a);if(!F.ok){const P=await F.text();throw new Error(`interrupt fetch failed: HTTP ${F.status} ${P}`)}const G=await tt(zA,F,`GET /v1/harness/sessions/${e}/interrupt`);v(G.pending)}catch(F){N(F instanceof Error?F.message:String(F))}};C.useEffect(()=>{D(),B(),V(),U(),R("loading");const F=new AbortController;let G=null,P=null,ae=!1,Se=!1,je=!1;const we=async()=>{for(;!ae&&!F.signal.aborted;)try{const xe=await Me(`/v1/harness/sessions/${e}/events`,t,{signal:F.signal,headers:{Accept:"text/event-stream"}},a);if(!xe.ok||xe.body===null){R("error"),G===null&&(G=window.setInterval(()=>{ae||je||(je=!0,U().finally(()=>{je=!1}))},5e3)),await new Promise(Ke=>{P=window.setTimeout(()=>Ke(),5e3)});continue}R("streaming");const ke=xe.body.getReader(),me=new TextDecoder;let Ge="";for(;!ae&&!F.signal.aborted;){const{value:Ke,done:rt}=await ke.read();if(rt)break;Ge+=me.decode(Ke,{stream:!0});const{events:nt,rest:qe}=Md(Ge,yw,"harness SSE event");Ge=qe;for(const Xt of nt)L(Xt)}}catch{if(F.signal.aborted)break;R("error"),G===null&&(G=window.setInterval(()=>{ae||je||(je=!0,U().finally(()=>{je=!1}))},5e3)),await new Promise(ke=>{P=window.setTimeout(()=>ke(),5e3)})}};try{we()}catch(xe){R("error"),N(xe instanceof Error?xe.message:String(xe))}const ve=window.setInterval(()=>{ae||Se||(Se=!0,Promise.all([D(),B(),V()]).finally(()=>{Se=!1}))},2e3);return()=>{ae=!0,F.abort(),window.clearInterval(ve),G!==null&&window.clearInterval(G),P!==null&&window.clearTimeout(P),R("closed")}},[e,t]);const te=async(F,G,P)=>{N(null),E(null);try{const ae=await Me(`/v1/harness/sessions/${e}/approvals/${encodeURIComponent(F)}`,t,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({decision:G,scope:P,decided_by:"ui"})},a);if(!ae.ok){const Se=await ae.text();throw new Error(`decide failed: HTTP ${ae.status} ${Se}`)}E(`Approval ${F} → ${G}/${P}`),B()}catch(ae){N(ae instanceof Error?ae.message:String(ae))}},z=async F=>{const G=F.trim();if(!G){N("Answer must not be empty");return}N(null),E(null),w(!0);try{const P=await Me(`/v1/harness/sessions/${e}/interrupt/answer`,t,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({answer:G})},a);if(!P.ok){const ae=await P.text();throw new Error(`answer failed: HTTP ${P.status} ${ae}`)}E("Answer submitted — session resuming."),v(null),D()}catch(P){N(P instanceof Error?P.message:String(P))}finally{w(!1)}},Y=async()=>{N(null),E(null);try{const F=await Me(`/v1/harness/sessions/${e}:cancel`,t,{method:"POST"},a);if(!F.ok){const G=await F.text();throw new Error(`cancel failed: HTTP ${F.status} ${G}`)}E("Cancel requested"),D()}catch(F){N(F instanceof Error?F.message:String(F))}},ce=async()=>{N(null),E(null),Z(!0);try{const F={mode:ee},G=oe.trim();G&&(F.user_input=G);const P=await Me(`/v1/harness/sessions/${e}:resume`,t,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify(F)},a);if(!P.ok){const ae=await P.text();throw new Error(`resume failed: HTTP ${P.status} ${ae}`)}ee==="rerun"?(E("Resume (rerun) — events reset, executor restarted."),I([]),j(null)):E("Resume (append) — prior events preserved, seq continues."),Q(""),D()}catch(F){N(F instanceof Error?F.message:String(F))}finally{Z(!1)}},de=C.useMemo(()=>c.find(F=>F.seq===k)??null,[c,k]),_e=$O(o);return d.jsxs("main",{className:"shell harness-detail-shell",children:[d.jsx(_a,{eyebrow:"Yanshi / Harness",title:`Session ${e.slice(0,8)}…`,activeSection:"harness",actions:d.jsxs(d.Fragment,{children:[d.jsx(ln,{className:"topbar-link",to:"/ui/harness/sessions",children:"← Sessions"}),d.jsx(ln,{className:"topbar-link topbar-cta",to:"/ui/harness/sessions/new",children:"+ New session"})]})}),d.jsxs("section",{className:"harness-controls",children:[d.jsxs("label",{children:[d.jsx("span",{children:"API token"}),d.jsx("input",{"data-testid":"harness-detail-token",type:"password",autoComplete:"off",value:t,onChange:F=>s(F.target.value),placeholder:"Bearer token (not persisted)"})]}),d.jsxs("span",{"data-testid":"harness-detail-stream-state",className:`stream-pill stream-${O}`,children:["stream: ",O]}),d.jsx("button",{"data-testid":"harness-detail-cancel",type:"button",onClick:()=>{Y()},disabled:_e,children:_e?"Terminal":"Cancel session"})]}),d.jsxs("section",{className:"harness-controls harness-resume-controls",children:[d.jsxs("label",{className:"harness-grow",children:[d.jsx("span",{children:"Resume prompt (optional — empty replays original)"}),d.jsx("input",{"data-testid":"harness-detail-resume-prompt",value:oe,onChange:F=>Q(F.target.value),placeholder:"Leave blank to rerun with the original prompt",disabled:!_e||q})]}),d.jsxs("label",{children:[d.jsx("span",{children:"Mode"}),d.jsxs("select",{"data-testid":"harness-detail-resume-mode",value:ee,onChange:F=>X(F.target.value),disabled:!_e||q,children:[d.jsx("option",{value:"rerun",children:"rerun (reset events)"}),d.jsx("option",{value:"append",children:"append (continue seq)"})]})]}),d.jsx("button",{"data-testid":"harness-detail-resume",type:"button",onClick:()=>{ce()},disabled:!_e||q,children:q?"Resuming…":`Resume (${ee})`})]}),A?d.jsx("p",{className:"error-line",children:A}):null,T?d.jsx("p",{className:"info-line",children:T}):null,d.jsxs("section",{className:"harness-detail-grid",children:[d.jsxs("section",{className:"harness-summary",children:[d.jsx("h2",{children:"Summary"}),o?d.jsxs("dl",{children:[d.jsxs("div",{children:[d.jsx("dt",{children:"Status"}),d.jsx("dd",{children:d.jsx("span",{className:`status-pill status-${Nw(o.status)}`,children:o.status})})]}),d.jsxs("div",{children:[d.jsx("dt",{children:"Tenant"}),d.jsx("dd",{children:o.tenant_id})]}),d.jsxs("div",{children:[d.jsx("dt",{children:"Profile"}),d.jsx("dd",{children:o.profile})]}),d.jsxs("div",{children:[d.jsx("dt",{children:"Runtime"}),d.jsx("dd",{children:o.runtime_kind})]}),d.jsxs("div",{children:[d.jsx("dt",{children:"Model"}),d.jsx("dd",{className:"harness-cell-mono",children:o.model})]}),o.skill_name?d.jsxs("div",{children:[d.jsx("dt",{children:"Skill"}),d.jsx("dd",{children:o.skill_name})]}):null,d.jsxs("div",{children:[d.jsx("dt",{children:"Workspace"}),d.jsx("dd",{className:"harness-cell-mono",children:o.workspace_root})]}),d.jsxs("div",{children:[d.jsx("dt",{children:"Started"}),d.jsx("dd",{children:Zn(o.started_at)})]}),d.jsxs("div",{children:[d.jsx("dt",{children:"Finished"}),d.jsx("dd",{children:Zn(o.finished_at)})]}),o.error?d.jsxs("div",{children:[d.jsx("dt",{children:"Error"}),d.jsx("dd",{className:"harness-cell-error",children:o.error})]}):null,o.final_answer?d.jsxs("div",{className:"harness-summary-answer",children:[d.jsx("dt",{children:"Final answer"}),d.jsx("dd",{children:d.jsx("pre",{children:o.final_answer})})]}):null,d.jsxs("div",{className:"harness-summary-prompt",children:[d.jsx("dt",{children:"Prompt"}),d.jsx("dd",{children:d.jsx("pre",{children:o.user_input})})]})]}):d.jsx("p",{children:"Loading…"})]}),b?d.jsx("section",{className:"harness-interrupt","aria-label":"Pending question","data-testid":"harness-interrupt-section",children:d.jsx(ZO,{pending:b,busy:S,onAnswer:F=>{z(F)}})}):null,d.jsxs("section",{className:"harness-approvals","aria-label":"Pending approvals","data-testid":"harness-approvals-section",children:[d.jsxs("h2",{children:["Pending approvals (",_.length,")"]}),_.length===0?d.jsx("p",{className:"harness-approvals-empty",children:"No approvals waiting for this session."}):d.jsx("ul",{className:"harness-approvals-list",children:_.map(F=>d.jsx(IO,{approval:F,onDecide:(G,P)=>{te(F.id,G,P)}},`${F.session_id}-${F.id}`))})]}),d.jsxs("section",{className:"harness-timeline","aria-label":"Event timeline",children:[d.jsxs("h2",{children:["Timeline (",c.length,")"]}),m?d.jsxs("p",{className:"harness-live-typing","data-testid":"harness-live-typing",children:["💭 ",m]}):null,c.length===0?d.jsx("p",{className:"harness-timeline-empty",children:"No events yet."}):d.jsx("ol",{className:"harness-event-list","data-testid":"harness-event-list",children:c.map(F=>d.jsxs("li",{className:`harness-event harness-event-${zl(F.kind)} ${k===F.seq?"harness-event-selected":""}`,onClick:()=>j(F.seq),children:[d.jsxs("span",{className:"harness-event-seq",children:["#",F.seq]}),d.jsx("span",{className:"harness-event-kind",children:F.kind}),d.jsx("span",{className:"harness-event-time",children:Zn(F.ts)})]},F.seq))})]}),d.jsxs("section",{className:"harness-event-payload",children:[d.jsx("h2",{children:"Event payload"}),de?d.jsx("pre",{children:As(de.payload)}):d.jsx("p",{children:"Select an event from the timeline."})]})]})]})}function IO({approval:e,onDecide:t}){const[s,a]=C.useState("once");return d.jsxs("li",{className:"harness-approval-card","data-testid":"harness-approval-card",children:[d.jsxs("header",{children:[d.jsx("strong",{children:e.tool}),d.jsx("span",{className:`risk-pill risk-${e.risk}`,children:e.risk})]}),d.jsx("p",{className:"harness-approval-reason",children:e.reason}),d.jsxs("p",{className:"harness-approval-meta",children:["step #",e.step_index," · ",e.idempotency??"unknown"," · raised"," ",Zn(e.requested_at)]}),e.params_summary!==void 0&&e.params_summary!==null?d.jsx("pre",{className:"harness-approval-params",children:As(e.params_summary)}):null,d.jsxs("div",{className:"harness-approval-controls",children:[d.jsxs("label",{children:[d.jsx("span",{children:"Scope"}),d.jsxs("select",{"data-testid":"harness-approval-scope",value:s,onChange:o=>a(o.target.value),children:[d.jsx("option",{value:"once",children:"once"}),d.jsx("option",{value:"session",children:"session"}),d.jsx("option",{value:"run",children:"run"})]})]}),d.jsx("button",{"data-testid":"harness-approval-allow",type:"button",className:"harness-btn harness-btn-allow",onClick:()=>t("allow",s),children:"Allow"}),d.jsx("button",{"data-testid":"harness-approval-deny",type:"button",className:"harness-btn harness-btn-deny",onClick:()=>t("deny",s),children:"Deny"}),d.jsx("button",{"data-testid":"harness-approval-deny-stop",type:"button",className:"harness-btn harness-btn-deny",onClick:()=>t("deny_and_stop",s),children:"Deny & Stop"})]})]})}function ZO({pending:e,busy:t,onAnswer:s}){const[a,o]=C.useState(""),l=()=>{!a.trim()||t||(s(a),o(""))};return d.jsxs("div",{className:"harness-interrupt-card","data-testid":"harness-interrupt-card",children:[d.jsxs("header",{children:[d.jsx("strong",{children:"❓ Question"}),d.jsxs("span",{className:"harness-interrupt-meta",children:["step #",e.step_index]})]}),d.jsx("p",{className:"harness-interrupt-question",children:e.question}),d.jsxs("div",{className:"harness-interrupt-controls",children:[d.jsx("textarea",{"data-testid":"harness-interrupt-answer",value:a,onChange:c=>o(c.target.value),placeholder:"Type your answer…",rows:3,disabled:t}),d.jsx("button",{"data-testid":"harness-interrupt-submit",type:"button",className:"harness-btn harness-btn-allow",onClick:l,disabled:t||!a.trim(),children:t?"Submitting…":"Submit answer"})]})]})}function PO({apiToken:e,onTokenChange:t}){const[s,a]=C.useState(()=>bt(ld,"default")),[o,l]=C.useState([]),[c,f]=C.useState(!1),[m,p]=C.useState(null);C.useEffect(()=>{vt(ld,s)},[s]);const _=async()=>{f(!0),p(null);try{const y=s.trim()||"default",b=await Me("/v1/harness/sessions",e,{},y);if(!b.ok){const w=await b.text();throw new Error(`list failed: HTTP ${b.status} ${w}`)}const v=await b.json(),S=AA.parse(v?.sessions??[]);l(S)}catch(y){p(ts(y))}finally{f(!1)}};return C.useEffect(()=>{let y=!1,b=!1;const v=async()=>{if(!(y||b)){b=!0;try{await _()}finally{b=!1}}};v();const S=window.setInterval(()=>{v()},4e3);return()=>{y=!0,window.clearInterval(S)}},[s,e]),d.jsxs("main",{className:"shell harness-list-shell",children:[d.jsx(_a,{eyebrow:"Yanshi / Harness",title:"Sessions",activeSection:"harness",actions:d.jsx(ln,{"data-testid":"harness-new-link",className:"topbar-link topbar-cta",to:"/ui/harness/sessions/new",children:"+ New session"})}),d.jsxs("section",{className:"harness-controls",children:[d.jsxs("label",{children:[d.jsx("span",{children:"Tenant"}),d.jsx("input",{"data-testid":"harness-list-tenant",value:s,onChange:y=>a(y.target.value),placeholder:"default"})]}),d.jsxs("label",{children:[d.jsx("span",{children:"API token"}),d.jsx("input",{"data-testid":"harness-list-token",type:"password",autoComplete:"off",value:e,onChange:y=>t(y.target.value),placeholder:"Bearer token (not persisted)"})]}),d.jsx("button",{type:"button",onClick:()=>{_()},disabled:c,children:c?"Loading…":"Refresh"})]}),m?d.jsx("p",{className:"error-line",children:m}):null,d.jsx("section",{className:"harness-table",children:d.jsxs("table",{"data-testid":"harness-list-table",children:[d.jsx("thead",{children:d.jsxs("tr",{children:[d.jsx("th",{children:"Started"}),d.jsx("th",{children:"Status"}),d.jsx("th",{children:"Profile"}),d.jsx("th",{children:"Runtime"}),d.jsx("th",{children:"Model"}),d.jsx("th",{children:"Prompt"}),d.jsx("th",{children:"ID"})]})}),d.jsx("tbody",{children:o.length===0?d.jsx("tr",{children:d.jsxs("td",{colSpan:7,className:"harness-table-empty",children:['No sessions yet for tenant "',s||"default",'". Use'," ",d.jsx(ln,{to:"/ui/harness/sessions/new",children:"+ New session"})," to create one."]})}):o.map(y=>d.jsxs("tr",{"data-testid":"harness-list-row",onClick:()=>{const b=s.trim(),v=b&&b!=="default"?`?tenant=${encodeURIComponent(b)}`:"";gs(`/ui/harness/sessions/${y.id}${v}`)},className:"harness-row",children:[d.jsx("td",{children:Zn(y.started_at)}),d.jsx("td",{children:d.jsx("span",{className:`status-pill status-${Nw(y.status)}`,children:y.status})}),d.jsx("td",{children:y.profile}),d.jsx("td",{children:y.runtime_kind}),d.jsx("td",{className:"harness-cell-mono",children:y.model}),d.jsx("td",{className:"harness-cell-prompt",children:y.user_input}),d.jsxs("td",{className:"harness-cell-mono",children:[y.id.slice(0,8),"…"]})]},y.id))})]})})]})}const YO="请用一句话总结当前工作区。",VO="/data/workspace",KO="";function GO({apiToken:e,onTokenChange:t}){const[s,a]=C.useState(()=>bt(ld,"default")),[o,l]=C.useState(()=>bt(Xv,YO)),[c,f]=C.useState(()=>bt(Fv,VO)),[m,p]=C.useState(()=>{const E=bt(Jv,"local");return t0.includes(E)?E:"local"}),[_,y]=C.useState(()=>{const E=bt(Qv,"react");return n0.includes(E)?E:"react"}),[b,v]=C.useState(()=>bt(Wv,KO)),[S,w]=C.useState(()=>bt(e0,"")),[k,j]=C.useState(!1),[A,N]=C.useState(null);C.useEffect(()=>{vt(ld,s)},[s]),C.useEffect(()=>{vt(Xv,o)},[o]),C.useEffect(()=>{vt(Fv,c)},[c]),C.useEffect(()=>{vt(Jv,m)},[m]),C.useEffect(()=>{vt(Qv,_)},[_]),C.useEffect(()=>{vt(Wv,b)},[b]),C.useEffect(()=>{vt(e0,S)},[S]);const T=async E=>{E.preventDefault(),N(null);const O=o.trim();if(!O){N("User prompt is required");return}const R=c.trim();if(!R){N("Workspace root is required");return}j(!0);try{const q={user_input:O,workspace_root:R,tenant_id:s.trim()||"default",profile:m,runtime_kind:_},Z=b.trim();Z&&(q.model=Z);const oe=S.trim();oe&&(q.skill_name=oe);const Q=q.tenant_id??"default",ee=await Me("/v1/harness/sessions",e,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify(q)},Q);if(!ee.ok){const I=await ee.text();throw new Error(`HTTP ${ee.status}: ${I}`)}const X=await ee.json();if(typeof X?.session_id!="string"||X.session_id.length===0)throw new Error("POST /v1/harness/sessions: response missing session_id (got: "+JSON.stringify(X)+")");const L=Q&&Q!=="default"?`?tenant=${encodeURIComponent(Q)}`:"";gs(`/ui/harness/sessions/${encodeURIComponent(X.session_id)}${L}`)}catch(q){N(q instanceof Error?q.message:String(q))}finally{j(!1)}};return d.jsxs("main",{className:"shell create-shell",children:[d.jsx(_a,{eyebrow:"Yanshi / Harness",title:"New session",activeSection:"harness",actions:d.jsx(ln,{className:"topbar-link",to:"/ui/harness/sessions",children:"← Sessions"})}),A?d.jsx("p",{className:"error-line",children:A}):null,d.jsxs("form",{className:"create-form",onSubmit:T,children:[d.jsxs("section",{className:"create-row",children:[d.jsxs("label",{children:[d.jsx("span",{children:"Tenant"}),d.jsx("input",{"data-testid":"harness-new-tenant",value:s,onChange:E=>a(E.target.value),placeholder:"default"})]}),d.jsxs("label",{children:[d.jsx("span",{children:"Profile"}),d.jsx("select",{"data-testid":"harness-new-profile",value:m,onChange:E=>p(E.target.value),children:t0.map(E=>d.jsx("option",{value:E,children:E},E))})]}),d.jsxs("label",{children:[d.jsx("span",{children:"Runtime"}),d.jsx("select",{"data-testid":"harness-new-runtime",value:_,onChange:E=>y(E.target.value),children:n0.map(E=>d.jsx("option",{value:E,children:E},E))})]}),d.jsxs("label",{children:[d.jsx("span",{children:"API token"}),d.jsx("input",{"data-testid":"harness-new-token",type:"password",autoComplete:"off",value:e,onChange:E=>t(E.target.value),placeholder:"Bearer token (not persisted)"})]})]}),d.jsxs("section",{className:"create-row",children:[d.jsxs("label",{className:"harness-grow",children:[d.jsx("span",{children:"Model (optional)"}),d.jsx("input",{"data-testid":"harness-new-model",value:b,onChange:E=>v(E.target.value),placeholder:"留空使用服务端默认模型（如 deepseek-chat）"})]}),d.jsxs("label",{className:"harness-grow",children:[d.jsx("span",{children:"Skill (optional)"}),d.jsx("input",{"data-testid":"harness-new-skill",value:S,onChange:E=>w(E.target.value),placeholder:"leave blank for no skill"})]}),d.jsxs("label",{className:"harness-grow",children:[d.jsx("span",{children:"Workspace root"}),d.jsx("input",{"data-testid":"harness-new-workspace",value:c,onChange:E=>f(E.target.value),placeholder:"/data/workspace (server) or /tmp (local)"})]})]}),d.jsxs("section",{className:"create-editor","aria-label":"User prompt editor",children:[d.jsx("div",{className:"pane-heading",children:d.jsx("span",{children:"Prompt"})}),d.jsx("textarea",{"data-testid":"harness-new-prompt",className:"code-editor",spellCheck:!0,value:o,onChange:E=>l(E.target.value),rows:10})]}),d.jsxs("footer",{className:"create-actions",children:[d.jsx("button",{"data-testid":"harness-new-submit",disabled:k,type:"submit",children:k?"Submitting…":"Start session"}),d.jsx("small",{children:"Inputs persist in localStorage (tenant, profile, runtime, model, skill, workspace, prompt). The API token is never saved."})]})]})]})}function XO(e){return typeof e!="number"||Number.isNaN(e)||e<0?"":e<1024?`${e} B`:e<1024*1024?`${(e/1024).toFixed(1)} KB`:`${(e/(1024*1024)).toFixed(2)} MB`}function ql({item:e,apiToken:t,tenantId:s,compact:a=!1}){const o=!!t.trim(),l=tc(e.apiPath),[c,f]=C.useState(null),[m,p]=C.useState(null),[_,y]=C.useState(null),[b,v]=C.useState(!1);C.useEffect(()=>{y(null),p(null);const A=e.kind==="subtitle";if(!o&&!A){f(null),v(!1);return}const N=new AbortController;let T=null;return v(!0),Me(e.apiPath,t,{signal:N.signal},s).then(async E=>{if(!E.ok)throw new Error(`HTTP ${E.status}`);if(A){const O=await E.text();p(O.slice(0,6e3))}else{const O=await E.blob();T=URL.createObjectURL(O),f(T)}}).catch(E=>{E instanceof DOMException&&E.name==="AbortError"||y(E instanceof Error?E.message:String(E))}).finally(()=>{v(!1)}),()=>{N.abort(),T&&URL.revokeObjectURL(T)}},[e.apiPath,e.kind,t,s,o]);const S=o?c:l,w=async()=>{if(!o){const A=document.createElement("a");A.href=l,A.download=e.label,A.target="_blank",A.rel="noopener noreferrer",A.click();return}try{const A=await Me(e.apiPath,t,{},s);if(!A.ok)throw new Error(`HTTP ${A.status}`);const N=await A.blob(),T=URL.createObjectURL(N),E=document.createElement("a");E.href=T,E.download=e.label,E.click(),window.setTimeout(()=>URL.revokeObjectURL(T),5e3)}catch(A){y(A instanceof Error?A.message:String(A))}},k={video:"视频 MP4",image:"幻灯片图像",html:"交互课件 HTML",audio:"口播音频",document:"文档课件",subtitle:"字幕时间轴"},j=XO(e.sizeBytes);return d.jsxs("article",{className:`studio-media-card ${a?"studio-media-card--compact":""}`,children:[d.jsxs("header",{className:"studio-media-card__header",children:[d.jsxs("div",{className:"studio-media-card__meta",children:[d.jsx("span",{className:`studio-media-badge studio-media-badge--${e.kind}`,children:k[e.kind]}),d.jsx("strong",{className:"studio-media-card__title",title:e.label,children:e.label}),d.jsxs("small",{className:"studio-media-card__sub",children:["#",e.nodeId,j?` · ${j}`:""]})]}),d.jsx("button",{type:"button",className:"studio-btn-ghost studio-btn-sm",onClick:()=>{w()},children:"下载 / 打开"})]}),d.jsxs("div",{className:"studio-media-card__body",children:[b&&d.jsx("p",{className:"studio-media-status",children:"正在加载预览…"}),_&&d.jsxs("p",{className:"studio-media-error",children:["预览加载失败：",_]}),!b&&!_&&e.kind==="image"&&S&&d.jsx("img",{className:"studio-media-img",src:S,alt:e.label,loading:"lazy"}),!b&&!_&&e.kind==="video"&&S&&d.jsx("video",{className:"studio-media-video",src:S,controls:!0,preload:"metadata"}),!b&&!_&&e.kind==="audio"&&S&&d.jsx("audio",{className:"studio-media-audio",src:S,controls:!0,preload:"metadata"}),!b&&!_&&e.kind==="html"&&S&&d.jsx("iframe",{className:"studio-media-iframe",src:S,title:e.label,sandbox:"allow-scripts"}),!b&&!_&&e.kind==="subtitle"&&m!==null&&d.jsx("pre",{className:"studio-media-subtitle",children:m}),!b&&!_&&e.kind==="document"&&d.jsx("div",{className:"studio-media-doc",children:d.jsxs("p",{children:["已生成可编辑文档产物（",e.label,"），点击右上角按钮下载或本地打开。"]})})]})]})}const FO={png:"image",jpg:"image",jpeg:"image",webp:"image",gif:"image",svg:"image",wav:"audio",mp3:"audio",ogg:"audio",m4a:"audio",mp4:"video",webm:"video",html:"html",htm:"html",pdf:"document",pptx:"document",docx:"document",srt:"subtitle",vtt:"subtitle"};function ud(e,t){const s=(t??"").toLowerCase();if(s.startsWith("image/"))return"image";if(s.startsWith("audio/"))return"audio";if(s.startsWith("video/"))return"video";if(s.startsWith("text/html"))return"html";if(s.includes("presentation")||s.includes("pdf")||s.includes("wordprocessing"))return"document";if(s.includes("subrip")||s.startsWith("text/vtt"))return"subtitle";const a=e.split("?")[0].split("#")[0],o=a.lastIndexOf(".");if(o<0)return null;const l=a.slice(o+1).toLowerCase();return FO[l]??null}function s0(e){return!e||e.length>1024||e.includes(`
`)||/^https?:\/\//i.test(e)?!1:e.includes("/artifacts/")||e.startsWith("artifacts/")?ud(e)!==null:!!((e.startsWith("/")||e.includes("/"))&&ud(e)!==null)}function vm(e){const t=e.replace(/\\/g,"/").split("/").filter(Boolean);return t[t.length-1]??e}function JO(e,t){return`/v1/runs/${encodeURIComponent(e)}/preview?path=${encodeURIComponent(t)}`}function Tw(e,t,s){if(!e||s===null||s===void 0)return[];const a=new Set,o=[],l=(m,p,_)=>{const y=m.trim();if(!y||a.has(y))return;const b=ud(y,_);if(!b)return;a.add(y);const v=JO(e,y);o.push({key:`${t}:${y}`,nodeId:t,label:p||vm(y),kind:b,apiPath:v,directUrl:tc(v),mimeType:_??void 0})},c=(m,p,_)=>{if(!(_>8||m===null||m===void 0)){if(typeof m=="string"){s0(m)&&l(m,`${vm(m)}`);return}if(Array.isArray(m)){m.forEach((y,b)=>c(y,`${p}[${b}]`,_+1));return}if(typeof m=="object"){const y=m;if(typeof y.path=="string"){const b=typeof y.mime_type=="string"?y.mime_type:null;(s0(y.path)||b)&&l(y.path,vm(y.path),b)}for(const[b,v]of Object.entries(y))c(v,p?`${p}.${b}`:b,_+1)}}};c(s,"",0);const f={video:0,image:1,html:2,audio:3,document:4,subtitle:5};return o.sort((m,p)=>f[m.kind]-f[p.kind]||m.key.localeCompare(p.key)),o}function QO(e,t,s){const a=[],o=new Set;for(const c of t){const f=ud(c.name,c.mime_type)??"document",m=`/v1/runs/${encodeURIComponent(e)}/artifacts/${encodeURIComponent(c.id)}`;o.add(`${c.node_id}:${c.name}`),a.push({key:`artifact:${c.id}`,nodeId:c.node_id,label:c.name,kind:f,apiPath:m,directUrl:tc(m),mimeType:c.mime_type??void 0,sizeBytes:c.size_bytes})}for(const[c,f]of Object.entries(s))for(const m of Tw(e,c,f)){const p=`${c}:${m.label}`;if(o.has(p)){const _=a.findIndex(y=>y.nodeId===c&&y.label===m.label);_>=0&&["video","image","html","audio"].includes(m.kind)&&(a[_]={...a[_],apiPath:m.apiPath,directUrl:m.directUrl});continue}o.add(p),a.push(m)}const l={video:0,html:1,document:2,image:3,audio:4,subtitle:5};return a.sort((c,f)=>l[c.kind]-l[f.kind]||c.label.localeCompare(f.label)),a}const Bh=Symbol.for("yaml.alias"),th=Symbol.for("yaml.document"),ma=Symbol.for("yaml.map"),Cw=Symbol.for("yaml.pair"),Js=Symbol.for("yaml.scalar"),Mr=Symbol.for("yaml.seq"),_s=Symbol.for("yaml.node.type"),go=e=>!!e&&typeof e=="object"&&e[_s]===Bh,yo=e=>!!e&&typeof e=="object"&&e[_s]===th,Pn=e=>!!e&&typeof e=="object"&&e[_s]===ma,At=e=>!!e&&typeof e=="object"&&e[_s]===Cw,xt=e=>!!e&&typeof e=="object"&&e[_s]===Js,$s=e=>!!e&&typeof e=="object"&&e[_s]===Mr;function $t(e){if(e&&typeof e=="object")switch(e[_s]){case ma:case Mr:return!0}return!1}function Rt(e){if(e&&typeof e=="object")switch(e[_s]){case Bh:case ma:case Js:case Mr:return!0}return!1}const Aw=e=>(xt(e)||$t(e))&&!!e.anchor,In=Symbol("break visit"),Ow=Symbol("skip children"),Gs=Symbol("remove node");function _o(e,t){const s=Mw(t);yo(e)?ar(null,e.contents,s,Object.freeze([e]))===Gs&&(e.contents=null):ar(null,e,s,Object.freeze([]))}_o.BREAK=In;_o.SKIP=Ow;_o.REMOVE=Gs;function ar(e,t,s,a){const o=zw(e,t,s,a);if(Rt(o)||At(o))return Dw(e,a,o),ar(e,o,s,a);if(typeof o!="symbol"){if($t(t)){a=Object.freeze(a.concat(t));for(let l=0;l<t.items.length;++l){const c=ar(l,t.items[l],s,a);if(typeof c=="number")l=c-1;else{if(c===In)return In;c===Gs&&(t.items.splice(l,1),l-=1)}}}else if(At(t)){a=Object.freeze(a.concat(t));const l=ar("key",t.key,s,a);if(l===In)return In;l===Gs&&(t.key=null);const c=ar("value",t.value,s,a);if(c===In)return In;c===Gs&&(t.value=null)}}return o}async function zd(e,t){const s=Mw(t);yo(e)?await or(null,e.contents,s,Object.freeze([e]))===Gs&&(e.contents=null):await or(null,e,s,Object.freeze([]))}zd.BREAK=In;zd.SKIP=Ow;zd.REMOVE=Gs;async function or(e,t,s,a){const o=await zw(e,t,s,a);if(Rt(o)||At(o))return Dw(e,a,o),or(e,o,s,a);if(typeof o!="symbol"){if($t(t)){a=Object.freeze(a.concat(t));for(let l=0;l<t.items.length;++l){const c=await or(l,t.items[l],s,a);if(typeof c=="number")l=c-1;else{if(c===In)return In;c===Gs&&(t.items.splice(l,1),l-=1)}}}else if(At(t)){a=Object.freeze(a.concat(t));const l=await or("key",t.key,s,a);if(l===In)return In;l===Gs&&(t.key=null);const c=await or("value",t.value,s,a);if(c===In)return In;c===Gs&&(t.value=null)}}return o}function Mw(e){return typeof e=="object"&&(e.Collection||e.Node||e.Value)?Object.assign({Alias:e.Node,Map:e.Node,Scalar:e.Node,Seq:e.Node},e.Value&&{Map:e.Value,Scalar:e.Value,Seq:e.Value},e.Collection&&{Map:e.Collection,Seq:e.Collection},e):e}function zw(e,t,s,a){if(typeof s=="function")return s(e,t,a);if(Pn(t))return s.Map?.(e,t,a);if($s(t))return s.Seq?.(e,t,a);if(At(t))return s.Pair?.(e,t,a);if(xt(t))return s.Scalar?.(e,t,a);if(go(t))return s.Alias?.(e,t,a)}function Dw(e,t,s){const a=t[t.length-1];if($t(a))a.items[e]=s;else if(At(a))e==="key"?a.key=s:a.value=s;else if(yo(a))a.contents=s;else{const o=go(a)?"alias":"scalar";throw new Error(`Cannot replace node with ${o} parent`)}}const WO={"!":"%21",",":"%2C","[":"%5B","]":"%5D","{":"%7B","}":"%7D"},eM=e=>e.replace(/[!,[\]{}]/g,t=>WO[t]);class Mn{constructor(t,s){this.docStart=null,this.docEnd=!1,this.yaml=Object.assign({},Mn.defaultYaml,t),this.tags=Object.assign({},Mn.defaultTags,s)}clone(){const t=new Mn(this.yaml,this.tags);return t.docStart=this.docStart,t}atDocument(){const t=new Mn(this.yaml,this.tags);switch(this.yaml.version){case"1.1":this.atNextDocument=!0;break;case"1.2":this.atNextDocument=!1,this.yaml={explicit:Mn.defaultYaml.explicit,version:"1.2"},this.tags=Object.assign({},Mn.defaultTags);break}return t}add(t,s){this.atNextDocument&&(this.yaml={explicit:Mn.defaultYaml.explicit,version:"1.1"},this.tags=Object.assign({},Mn.defaultTags),this.atNextDocument=!1);const a=t.trim().split(/[ \t]+/),o=a.shift();switch(o){case"%TAG":{if(a.length!==2&&(s(0,"%TAG directive should contain exactly two parts"),a.length<2))return!1;const[l,c]=a;return this.tags[l]=c,!0}case"%YAML":{if(this.yaml.explicit=!0,a.length!==1)return s(0,"%YAML directive should contain exactly one part"),!1;const[l]=a;if(l==="1.1"||l==="1.2")return this.yaml.version=l,!0;{const c=/^\d+\.\d+$/.test(l);return s(6,`Unsupported YAML version ${l}`,c),!1}}default:return s(0,`Unknown directive ${o}`,!0),!1}}tagName(t,s){if(t==="!")return"!";if(t[0]!=="!")return s(`Not a valid tag: ${t}`),null;if(t[1]==="<"){const c=t.slice(2,-1);return c==="!"||c==="!!"?(s(`Verbatim tags aren't resolved, so ${t} is invalid.`),null):(t[t.length-1]!==">"&&s("Verbatim tags must end with a >"),c)}const[,a,o]=t.match(/^(.*!)([^!]*)$/s);o||s(`The ${t} tag has no suffix`);const l=this.tags[a];if(l)try{return l+decodeURIComponent(o)}catch(c){return s(String(c)),null}return a==="!"?t:(s(`Could not resolve tag: ${t}`),null)}tagString(t){for(const[s,a]of Object.entries(this.tags))if(t.startsWith(a))return s+eM(t.substring(a.length));return t[0]==="!"?t:`!<${t}>`}toString(t){const s=this.yaml.explicit?[`%YAML ${this.yaml.version||"1.2"}`]:[],a=Object.entries(this.tags);let o;if(t&&a.length>0&&Rt(t.contents)){const l={};_o(t.contents,(c,f)=>{Rt(f)&&f.tag&&(l[f.tag]=!0)}),o=Object.keys(l)}else o=[];for(const[l,c]of a)l==="!!"&&c==="tag:yaml.org,2002:"||(!t||o.some(f=>f.startsWith(c)))&&s.push(`%TAG ${l} ${c}`);return s.join(`
`)}}Mn.defaultYaml={explicit:!1,version:"1.2"};Mn.defaultTags={"!!":"tag:yaml.org,2002:"};function $w(e){if(/[\x00-\x19\s,[\]{}]/.test(e)){const s=`Anchor must not contain whitespace or control characters: ${JSON.stringify(e)}`;throw new Error(s)}return!0}function Rw(e){const t=new Set;return _o(e,{Value(s,a){a.anchor&&t.add(a.anchor)}}),t}function Lw(e,t){for(let s=1;;++s){const a=`${e}${s}`;if(!t.has(a))return a}}function tM(e,t){const s=[],a=new Map;let o=null;return{onAnchor:l=>{s.push(l),o??(o=Rw(e));const c=Lw(t,o);return o.add(c),c},setAnchors:()=>{for(const l of s){const c=a.get(l);if(typeof c=="object"&&c.anchor&&(xt(c.node)||$t(c.node)))c.node.anchor=c.anchor;else{const f=new Error("Failed to resolve repeated object (this should not happen)");throw f.source=l,f}}},sourceObjects:a}}function rr(e,t,s,a){if(a&&typeof a=="object")if(Array.isArray(a))for(let o=0,l=a.length;o<l;++o){const c=a[o],f=rr(e,a,String(o),c);f===void 0?delete a[o]:f!==c&&(a[o]=f)}else if(a instanceof Map)for(const o of Array.from(a.keys())){const l=a.get(o),c=rr(e,a,o,l);c===void 0?a.delete(o):c!==l&&a.set(o,c)}else if(a instanceof Set)for(const o of Array.from(a)){const l=rr(e,a,o,o);l===void 0?a.delete(o):l!==o&&(a.delete(o),a.add(l))}else for(const[o,l]of Object.entries(a)){const c=rr(e,a,o,l);c===void 0?delete a[o]:c!==l&&(a[o]=c)}return e.call(t,s,a)}function ys(e,t,s){if(Array.isArray(e))return e.map((a,o)=>ys(a,String(o),s));if(e&&typeof e.toJSON=="function"){if(!s||!Aw(e))return e.toJSON(t,s);const a={aliasCount:0,count:1,res:void 0};s.anchors.set(e,a),s.onCreate=l=>{a.res=l,delete s.onCreate};const o=e.toJSON(t,s);return s.onCreate&&s.onCreate(o),o}return typeof e=="bigint"&&!s?.keep?Number(e):e}class Hh{constructor(t){Object.defineProperty(this,_s,{value:t})}clone(){const t=Object.create(Object.getPrototypeOf(this),Object.getOwnPropertyDescriptors(this));return this.range&&(t.range=this.range.slice()),t}toJS(t,{mapAsMap:s,maxAliasCount:a,onAnchor:o,reviver:l}={}){if(!yo(t))throw new TypeError("A document argument is required");const c={anchors:new Map,doc:t,keep:!0,mapAsMap:s===!0,mapKeyWarned:!1,maxAliasCount:typeof a=="number"?a:100},f=ys(this,"",c);if(typeof o=="function")for(const{count:m,res:p}of c.anchors.values())o(p,m);return typeof l=="function"?rr(l,{"":f},"",f):f}}class Dd extends Hh{constructor(t){super(Bh),this.source=t,Object.defineProperty(this,"tag",{set(){throw new Error("Alias nodes cannot have tags")}})}resolve(t,s){if(s?.maxAliasCount===0)throw new ReferenceError("Alias resolution is disabled");let a;s?.aliasResolveCache?a=s.aliasResolveCache:(a=[],_o(t,{Node:(l,c)=>{(go(c)||Aw(c))&&a.push(c)}}),s&&(s.aliasResolveCache=a));let o;for(const l of a){if(l===this)break;l.anchor===this.source&&(o=l)}if(o&&s){const{anchors:l,doc:c,maxAliasCount:f}=s;let m=l.get(o);if(m||(ys(o,null,s),m=l.get(o)),m?.res===void 0){const p="This should not happen: Alias anchor was not resolved?";throw new ReferenceError(p)}if(f>=0&&(m.count+=1,m.aliasCount===0&&(m.aliasCount=Pu(c,o,l)),m.count*m.aliasCount>f)){const p="Excessive alias count indicates a resource exhaustion attack";throw new ReferenceError(p)}}return o}toJSON(t,s){if(!s)return{source:this.source};const a=this.resolve(s.doc,s);if(!a){const o=`Unresolved alias (the anchor must be set before the alias): ${this.source}`;throw new ReferenceError(o)}return s.anchors.get(a).res}toString(t,s,a){const o=`*${this.source}`;if(t){if($w(this.source),t.options.verifyAliasOrder&&!t.anchors.has(this.source)){const l=`Unresolved alias (the anchor must be set before the alias): ${this.source}`;throw new Error(l)}if(t.implicitKey)return`${o} `}return o}}function Pu(e,t,s){if(go(t)){const a=t.resolve(e),o=s&&a&&s.get(a);return o?o.count*o.aliasCount:0}else if($t(t)){let a=0;for(const o of t.items){const l=Pu(e,o,s);l>a&&(a=l)}return a}else if(At(t)){const a=Pu(e,t.key,s),o=Pu(e,t.value,s);return Math.max(a,o)}return 1}const qw=e=>!e||typeof e!="function"&&typeof e!="object";class Ze extends Hh{constructor(t){super(Js),this.value=t}toJSON(t,s){return s?.keep?this.value:ys(this.value,t,s)}toString(){return String(this.value)}}Ze.BLOCK_FOLDED="BLOCK_FOLDED";Ze.BLOCK_LITERAL="BLOCK_LITERAL";Ze.PLAIN="PLAIN";Ze.QUOTE_DOUBLE="QUOTE_DOUBLE";Ze.QUOTE_SINGLE="QUOTE_SINGLE";const nM="tag:yaml.org,2002:";function sM(e,t,s){if(t){const a=s.filter(l=>l.tag===t),o=a.find(l=>!l.format)??a[0];if(!o)throw new Error(`Tag ${t} not found`);return o}return s.find(a=>a.identify?.(e)&&!a.format)}function Ul(e,t,s){if(yo(e)&&(e=e.contents),Rt(e))return e;if(At(e)){const y=s.schema[ma].createNode?.(s.schema,null,s);return y.items.push(e),y}(e instanceof String||e instanceof Number||e instanceof Boolean||typeof BigInt<"u"&&e instanceof BigInt)&&(e=e.valueOf());const{aliasDuplicateObjects:a,onAnchor:o,onTagObj:l,schema:c,sourceObjects:f}=s;let m;if(a&&e&&typeof e=="object"){if(m=f.get(e),m)return m.anchor??(m.anchor=o(e)),new Dd(m.anchor);m={anchor:null,node:null},f.set(e,m)}t?.startsWith("!!")&&(t=nM+t.slice(2));let p=sM(e,t,c.tags);if(!p){if(e&&typeof e.toJSON=="function"&&(e=e.toJSON()),!e||typeof e!="object"){const y=new Ze(e);return m&&(m.node=y),y}p=e instanceof Map?c[ma]:Symbol.iterator in Object(e)?c[Mr]:c[ma]}l&&(l(p),delete s.onTagObj);const _=p?.createNode?p.createNode(s.schema,e,s):typeof p?.nodeClass?.from=="function"?p.nodeClass.from(s.schema,e,s):new Ze(e);return t?_.tag=t:p.default||(_.tag=p.tag),m&&(m.node=_),_}function dd(e,t,s){let a=s;for(let o=t.length-1;o>=0;--o){const l=t[o];if(typeof l=="number"&&Number.isInteger(l)&&l>=0){const c=[];c[l]=a,a=c}else a=new Map([[l,a]])}return Ul(a,void 0,{aliasDuplicateObjects:!1,keepUndefined:!1,onAnchor:()=>{throw new Error("This should not happen, please report a bug.")},schema:e,sourceObjects:new Map})}const Cl=e=>e==null||typeof e=="object"&&!!e[Symbol.iterator]().next().done;class Uw extends Hh{constructor(t,s){super(t),Object.defineProperty(this,"schema",{value:s,configurable:!0,enumerable:!1,writable:!0})}clone(t){const s=Object.create(Object.getPrototypeOf(this),Object.getOwnPropertyDescriptors(this));return t&&(s.schema=t),s.items=s.items.map(a=>Rt(a)||At(a)?a.clone(t):a),this.range&&(s.range=this.range.slice()),s}addIn(t,s){if(Cl(t))this.add(s);else{const[a,...o]=t,l=this.get(a,!0);if($t(l))l.addIn(o,s);else if(l===void 0&&this.schema)this.set(a,dd(this.schema,o,s));else throw new Error(`Expected YAML collection at ${a}. Remaining path: ${o}`)}}deleteIn(t){const[s,...a]=t;if(a.length===0)return this.delete(s);const o=this.get(s,!0);if($t(o))return o.deleteIn(a);throw new Error(`Expected YAML collection at ${s}. Remaining path: ${a}`)}getIn(t,s){const[a,...o]=t,l=this.get(a,!0);return o.length===0?!s&&xt(l)?l.value:l:$t(l)?l.getIn(o,s):void 0}hasAllNullValues(t){return this.items.every(s=>{if(!At(s))return!1;const a=s.value;return a==null||t&&xt(a)&&a.value==null&&!a.commentBefore&&!a.comment&&!a.tag})}hasIn(t){const[s,...a]=t;if(a.length===0)return this.has(s);const o=this.get(s,!0);return $t(o)?o.hasIn(a):!1}setIn(t,s){const[a,...o]=t;if(o.length===0)this.set(a,s);else{const l=this.get(a,!0);if($t(l))l.setIn(o,s);else if(l===void 0&&this.schema)this.set(a,dd(this.schema,o,s));else throw new Error(`Expected YAML collection at ${a}. Remaining path: ${o}`)}}}const iM=e=>e.replace(/^(?!$)(?: $)?/gm,"#");function Si(e,t){return/^\n+$/.test(e)?e.substring(1):t?e.replace(/^(?! *$)/gm,t):e}const Fa=(e,t,s)=>e.endsWith(`
`)?Si(s,t):s.includes(`
`)?`
`+Si(s,t):(e.endsWith(" ")?"":" ")+s,Bw="flow",nh="block",Yu="quoted";function $d(e,t,s="flow",{indentAtStart:a,lineWidth:o=80,minContentWidth:l=20,onFold:c,onOverflow:f}={}){if(!o||o<0)return e;o<l&&(l=0);const m=Math.max(1+l,1+o-t.length);if(e.length<=m)return e;const p=[],_={};let y=o-t.length;typeof a=="number"&&(a>o-Math.max(2,l)?p.push(0):y=o-a);let b,v,S=!1,w=-1,k=-1,j=-1;s===nh&&(w=i0(e,w,t.length),w!==-1&&(y=w+m));for(let N;N=e[w+=1];){if(s===Yu&&N==="\\"){switch(k=w,e[w+1]){case"x":w+=3;break;case"u":w+=5;break;case"U":w+=9;break;default:w+=1}j=w}if(N===`
`)s===nh&&(w=i0(e,w,t.length)),y=w+t.length+m,b=void 0;else{if(N===" "&&v&&v!==" "&&v!==`
`&&v!=="	"){const T=e[w+1];T&&T!==" "&&T!==`
`&&T!=="	"&&(b=w)}if(w>=y)if(b)p.push(b),y=b+m,b=void 0;else if(s===Yu){for(;v===" "||v==="	";)v=N,N=e[w+=1],S=!0;const T=w>j+1?w-2:k-1;if(_[T])return e;p.push(T),_[T]=!0,y=T+m,b=void 0}else S=!0}v=N}if(S&&f&&f(),p.length===0)return e;c&&c();let A=e.slice(0,p[0]);for(let N=0;N<p.length;++N){const T=p[N],E=p[N+1]||e.length;T===0?A=`
${t}${e.slice(0,E)}`:(s===Yu&&_[T]&&(A+=`${e[T]}\\`),A+=`
${t}${e.slice(T+1,E)}`)}return A}function i0(e,t,s){let a=t,o=t+1,l=e[o];for(;l===" "||l==="	";)if(t<o+s)l=e[++t];else{do l=e[++t];while(l&&l!==`
`);a=t,o=t+1,l=e[o]}return a}const Rd=(e,t)=>({indentAtStart:t?e.indent.length:e.indentAtStart,lineWidth:e.options.lineWidth,minContentWidth:e.options.minContentWidth}),Ld=e=>/^(%|---|\.\.\.)/m.test(e);function aM(e,t,s){if(!t||t<0)return!1;const a=t-s,o=e.length;if(o<=a)return!1;for(let l=0,c=0;l<o;++l)if(e[l]===`
`){if(l-c>a)return!0;if(c=l+1,o-c<=a)return!1}return!0}function Dl(e,t){const s=JSON.stringify(e);if(t.options.doubleQuotedAsJSON)return s;const{implicitKey:a}=t,o=t.options.doubleQuotedMinMultiLineLength,l=t.indent||(Ld(e)?"  ":"");let c="",f=0;for(let m=0,p=s[m];p;p=s[++m])if(p===" "&&s[m+1]==="\\"&&s[m+2]==="n"&&(c+=s.slice(f,m)+"\\ ",m+=1,f=m,p="\\"),p==="\\")switch(s[m+1]){case"u":{c+=s.slice(f,m);const _=s.substr(m+2,4);switch(_){case"0000":c+="\\0";break;case"0007":c+="\\a";break;case"000b":c+="\\v";break;case"001b":c+="\\e";break;case"0085":c+="\\N";break;case"00a0":c+="\\_";break;case"2028":c+="\\L";break;case"2029":c+="\\P";break;default:_.substr(0,2)==="00"?c+="\\x"+_.substr(2):c+=s.substr(m,6)}m+=5,f=m+1}break;case"n":if(a||s[m+2]==='"'||s.length<o)m+=1;else{for(c+=s.slice(f,m)+`

`;s[m+2]==="\\"&&s[m+3]==="n"&&s[m+4]!=='"';)c+=`
`,m+=2;c+=l,s[m+2]===" "&&(c+="\\"),m+=1,f=m+1}break;default:m+=1}return c=f?c+s.slice(f):s,a?c:$d(c,l,Yu,Rd(t,!1))}function sh(e,t){if(t.options.singleQuote===!1||t.implicitKey&&e.includes(`
`)||/[ \t]\n|\n[ \t]/.test(e))return Dl(e,t);const s=t.indent||(Ld(e)?"  ":""),a="'"+e.replace(/'/g,"''").replace(/\n+/g,`$&
${s}`)+"'";return t.implicitKey?a:$d(a,s,Bw,Rd(t,!1))}function lr(e,t){const{singleQuote:s}=t.options;let a;if(s===!1)a=Dl;else{const o=e.includes('"'),l=e.includes("'");o&&!l?a=sh:l&&!o?a=Dl:a=s?sh:Dl}return a(e,t)}let ih;try{ih=new RegExp(`(^|(?<!
))
+(?!
|$)`,"g")}catch{ih=/\n+(?!\n|$)/g}function Vu({comment:e,type:t,value:s},a,o,l){const{blockQuote:c,commentString:f,lineWidth:m}=a.options;if(!c||/\n[\t ]+$/.test(s))return lr(s,a);const p=a.indent||(a.forceBlockIndent||Ld(s)?"  ":""),_=c==="literal"?!0:c==="folded"||t===Ze.BLOCK_FOLDED?!1:t===Ze.BLOCK_LITERAL?!0:!aM(s,m,p.length);if(!s)return _?`|
`:`>
`;let y,b;for(b=s.length;b>0;--b){const E=s[b-1];if(E!==`
`&&E!=="	"&&E!==" ")break}let v=s.substring(b);const S=v.indexOf(`
`);S===-1?y="-":s===v||S!==v.length-1?(y="+",l&&l()):y="",v&&(s=s.slice(0,-v.length),v[v.length-1]===`
`&&(v=v.slice(0,-1)),v=v.replace(ih,`$&${p}`));let w=!1,k,j=-1;for(k=0;k<s.length;++k){const E=s[k];if(E===" ")w=!0;else if(E===`
`)j=k;else break}let A=s.substring(0,j<k?j+1:k);A&&(s=s.substring(A.length),A=A.replace(/\n+/g,`$&${p}`));let T=(w?p?"2":"1":"")+y;if(e&&(T+=" "+f(e.replace(/ ?[\r\n]+/g," ")),o&&o()),!_){const E=s.replace(/\n+/g,`
$&`).replace(/(?:^|\n)([\t ].*)(?:([\n\t ]*)\n(?![\n\t ]))?/g,"$1$2").replace(/\n+/g,`$&${p}`);let O=!1;const R=Rd(a,!0);c!=="folded"&&t!==Ze.BLOCK_FOLDED&&(R.onOverflow=()=>{O=!0});const q=$d(`${A}${E}${v}`,p,nh,R);if(!O)return`>${T}
${p}${q}`}return s=s.replace(/\n+/g,`$&${p}`),`|${T}
${p}${A}${s}${v}`}function oM(e,t,s,a){const{type:o,value:l}=e,{actualString:c,implicitKey:f,indent:m,indentStep:p,inFlow:_}=t;if(f&&l.includes(`
`)||_&&/[[\]{},]/.test(l))return lr(l,t);if(/^[\n\t ,[\]{}#&*!|>'"%@`]|^[?-]$|^[?-][ \t]|[\n:][ \t]|[ \t]\n|[\n\t ]#|[\n\t :]$/.test(l))return f||_||!l.includes(`
`)?lr(l,t):Vu(e,t,s,a);if(!f&&!_&&o!==Ze.PLAIN&&l.includes(`
`))return Vu(e,t,s,a);if(Ld(l)){if(m==="")return t.forceBlockIndent=!0,Vu(e,t,s,a);if(f&&m===p)return lr(l,t)}const y=l.replace(/\n+/g,`$&
${m}`);if(c){const b=w=>w.default&&w.tag!=="tag:yaml.org,2002:str"&&w.test?.test(y),{compat:v,tags:S}=t.doc.schema;if(S.some(b)||v?.some(b))return lr(l,t)}return f?y:$d(y,m,Bw,Rd(t,!1))}function nc(e,t,s,a){const{implicitKey:o,inFlow:l}=t,c=typeof e.value=="string"?e:Object.assign({},e,{value:String(e.value)});let{type:f}=e;f!==Ze.QUOTE_DOUBLE&&/[\x00-\x08\x0b-\x1f\x7f-\x9f\u{D800}-\u{DFFF}]/u.test(c.value)&&(f=Ze.QUOTE_DOUBLE);const m=_=>{switch(_){case Ze.BLOCK_FOLDED:case Ze.BLOCK_LITERAL:return o||l?lr(c.value,t):Vu(c,t,s,a);case Ze.QUOTE_DOUBLE:return Dl(c.value,t);case Ze.QUOTE_SINGLE:return sh(c.value,t);case Ze.PLAIN:return oM(c,t,s,a);default:return null}};let p=m(f);if(p===null){const{defaultKeyType:_,defaultStringType:y}=t.options,b=o&&_||y;if(p=m(b),p===null)throw new Error(`Unsupported default string type ${b}`)}return p}function Hw(e,t){const s=Object.assign({blockQuote:!0,commentString:iM,defaultKeyType:null,defaultStringType:"PLAIN",directives:null,doubleQuotedAsJSON:!1,doubleQuotedMinMultiLineLength:40,falseStr:"false",flowCollectionPadding:!0,indentSeq:!0,lineWidth:80,minContentWidth:20,nullStr:"null",simpleKeys:!1,singleQuote:null,trailingComma:!1,trueStr:"true",verifyAliasOrder:!0},e.schema.toStringOptions,t);let a;switch(s.collectionStyle){case"block":a=!1;break;case"flow":a=!0;break;default:a=null}return{anchors:new Set,doc:e,flowCollectionPadding:s.flowCollectionPadding?" ":"",indent:"",indentStep:typeof s.indent=="number"?" ".repeat(s.indent):"  ",inFlow:a,options:s}}function rM(e,t){if(t.tag){const o=e.filter(l=>l.tag===t.tag);if(o.length>0)return o.find(l=>l.format===t.format)??o[0]}let s,a;if(xt(t)){a=t.value;let o=e.filter(l=>l.identify?.(a));if(o.length>1){const l=o.filter(c=>c.test);l.length>0&&(o=l)}s=o.find(l=>l.format===t.format)??o.find(l=>!l.format)}else a=t,s=e.find(o=>o.nodeClass&&a instanceof o.nodeClass);if(!s){const o=a?.constructor?.name??(a===null?"null":typeof a);throw new Error(`Tag not resolved for ${o} value`)}return s}function lM(e,t,{anchors:s,doc:a}){if(!a.directives)return"";const o=[],l=(xt(e)||$t(e))&&e.anchor;l&&$w(l)&&(s.add(l),o.push(`&${l}`));const c=e.tag??(t.default?null:t.tag);return c&&o.push(a.directives.tagString(c)),o.join(" ")}function br(e,t,s,a){if(At(e))return e.toString(t,s,a);if(go(e)){if(t.doc.directives)return e.toString(t);if(t.resolvedAliases?.has(e))throw new TypeError("Cannot stringify circular structure without alias nodes");t.resolvedAliases?t.resolvedAliases.add(e):t.resolvedAliases=new Set([e]),e=e.resolve(t.doc)}let o;const l=Rt(e)?e:t.doc.createNode(e,{onTagObj:m=>o=m});o??(o=rM(t.doc.schema.tags,l));const c=lM(l,o,t);c.length>0&&(t.indentAtStart=(t.indentAtStart??0)+c.length+1);const f=typeof o.stringify=="function"?o.stringify(l,t,s,a):xt(l)?nc(l,t,s,a):l.toString(t,s,a);return c?xt(l)||f[0]==="{"||f[0]==="["?`${c} ${f}`:`${c}
${t.indent}${f}`:f}function cM({key:e,value:t},s,a,o){const{allNullValues:l,doc:c,indent:f,indentStep:m,options:{commentString:p,indentSeq:_,simpleKeys:y}}=s;let b=Rt(e)&&e.comment||null;if(y){if(b)throw new Error("With simple keys, key nodes cannot have comments");if($t(e)||!Rt(e)&&typeof e=="object"){const R="With simple keys, collection cannot be used as a key value";throw new Error(R)}}let v=!y&&(!e||b&&t==null&&!s.inFlow||$t(e)||(xt(e)?e.type===Ze.BLOCK_FOLDED||e.type===Ze.BLOCK_LITERAL:typeof e=="object"));s=Object.assign({},s,{allNullValues:!1,implicitKey:!v&&(y||!l),indent:f+m});let S=!1,w=!1,k=br(e,s,()=>S=!0,()=>w=!0);if(!v&&!s.inFlow&&k.length>1024){if(y)throw new Error("With simple keys, single line scalar must not span more than 1024 characters");v=!0}if(s.inFlow){if(l||t==null)return S&&a&&a(),k===""?"?":v?`? ${k}`:k}else if(l&&!y||t==null&&v)return k=`? ${k}`,b&&!S?k+=Fa(k,s.indent,p(b)):w&&o&&o(),k;S&&(b=null),v?(b&&(k+=Fa(k,s.indent,p(b))),k=`? ${k}
${f}:`):(k=`${k}:`,b&&(k+=Fa(k,s.indent,p(b))));let j,A,N;Rt(t)?(j=!!t.spaceBefore,A=t.commentBefore,N=t.comment):(j=!1,A=null,N=null,t&&typeof t=="object"&&(t=c.createNode(t))),s.implicitKey=!1,!v&&!b&&xt(t)&&(s.indentAtStart=k.length+1),w=!1,!_&&m.length>=2&&!s.inFlow&&!v&&$s(t)&&!t.flow&&!t.tag&&!t.anchor&&(s.indent=s.indent.substring(2));let T=!1;const E=br(t,s,()=>T=!0,()=>w=!0);let O=" ";if(b||j||A){if(O=j?`
`:"",A){const R=p(A);O+=`
${Si(R,s.indent)}`}E===""&&!s.inFlow?O===`
`&&N&&(O=`

`):O+=`
${s.indent}`}else if(!v&&$t(t)){const R=E[0],q=E.indexOf(`
`),Z=q!==-1,oe=s.inFlow??t.flow??t.items.length===0;if(Z||!oe){let Q=!1;if(Z&&(R==="&"||R==="!")){let ee=E.indexOf(" ");R==="&"&&ee!==-1&&ee<q&&E[ee+1]==="!"&&(ee=E.indexOf(" ",ee+1)),(ee===-1||q<ee)&&(Q=!0)}Q||(O=`
${s.indent}`)}}else(E===""||E[0]===`
`)&&(O="");return k+=O+E,s.inFlow?T&&a&&a():N&&!T?k+=Fa(k,s.indent,p(N)):w&&o&&o(),k}function Iw(e,t){(e==="debug"||e==="warn")&&console.warn(t)}const Cu="<<",Ei={identify:e=>e===Cu||typeof e=="symbol"&&e.description===Cu,default:"key",tag:"tag:yaml.org,2002:merge",test:/^<<$/,resolve:()=>Object.assign(new Ze(Symbol(Cu)),{addToJSMap:Zw}),stringify:()=>Cu},uM=(e,t)=>(Ei.identify(t)||xt(t)&&(!t.type||t.type===Ze.PLAIN)&&Ei.identify(t.value))&&e?.doc.schema.tags.some(s=>s.tag===Ei.tag&&s.default);function Zw(e,t,s){const a=Pw(e,s);if($s(a))for(const o of a.items)xm(e,t,o);else if(Array.isArray(a))for(const o of a)xm(e,t,o);else xm(e,t,a)}function xm(e,t,s){const a=Pw(e,s);if(!Pn(a))throw new Error("Merge sources must be maps or map aliases");const o=a.toJSON(null,e,Map);for(const[l,c]of o)t instanceof Map?t.has(l)||t.set(l,c):t instanceof Set?t.add(l):Object.prototype.hasOwnProperty.call(t,l)||Object.defineProperty(t,l,{value:c,writable:!0,enumerable:!0,configurable:!0});return t}function Pw(e,t){return e&&go(t)?t.resolve(e.doc,e):t}function Yw(e,t,{key:s,value:a}){if(Rt(s)&&s.addToJSMap)s.addToJSMap(e,t,a);else if(uM(e,s))Zw(e,t,a);else{const o=ys(s,"",e);if(t instanceof Map)t.set(o,ys(a,o,e));else if(t instanceof Set)t.add(o);else{const l=dM(s,o,e),c=ys(a,l,e);l in t?Object.defineProperty(t,l,{value:c,writable:!0,enumerable:!0,configurable:!0}):t[l]=c}}return t}function dM(e,t,s){if(t===null)return"";if(typeof t!="object")return String(t);if(Rt(e)&&s?.doc){const a=Hw(s.doc,{});a.anchors=new Set;for(const l of s.anchors.keys())a.anchors.add(l.anchor);a.inFlow=!0,a.inStringifyKey=!0;const o=e.toString(a);if(!s.mapKeyWarned){let l=JSON.stringify(o);l.length>40&&(l=l.substring(0,36)+'..."'),Iw(s.doc.options.logLevel,`Keys with collection values will be stringified due to JS Object restrictions: ${l}. Set mapAsMap: true to use object keys.`),s.mapKeyWarned=!0}return o}return JSON.stringify(t)}function Ih(e,t,s){const a=Ul(e,void 0,s),o=Ul(t,void 0,s);return new jn(a,o)}class jn{constructor(t,s=null){Object.defineProperty(this,_s,{value:Cw}),this.key=t,this.value=s}clone(t){let{key:s,value:a}=this;return Rt(s)&&(s=s.clone(t)),Rt(a)&&(a=a.clone(t)),new jn(s,a)}toJSON(t,s){const a=s?.mapAsMap?new Map:{};return Yw(s,a,this)}toString(t,s,a){return t?.doc?cM(this,t,s,a):JSON.stringify(this)}}function Vw(e,t,s){return(t.inFlow??e.flow?pM:fM)(e,t,s)}function fM({comment:e,items:t},s,{blockItemPrefix:a,flowChars:o,itemIndent:l,onChompKeep:c,onComment:f}){const{indent:m,options:{commentString:p}}=s,_=Object.assign({},s,{indent:l,type:null});let y=!1;const b=[];for(let S=0;S<t.length;++S){const w=t[S];let k=null;if(Rt(w))!y&&w.spaceBefore&&b.push(""),fd(s,b,w.commentBefore,y),w.comment&&(k=w.comment);else if(At(w)){const A=Rt(w.key)?w.key:null;A&&(!y&&A.spaceBefore&&b.push(""),fd(s,b,A.commentBefore,y))}y=!1;let j=br(w,_,()=>k=null,()=>y=!0);k&&(j+=Fa(j,l,p(k))),y&&k&&(y=!1),b.push(a+j)}let v;if(b.length===0)v=o.start+o.end;else{v=b[0];for(let S=1;S<b.length;++S){const w=b[S];v+=w?`
${m}${w}`:`
`}}return e?(v+=`
`+Si(p(e),m),f&&f()):y&&c&&c(),v}function pM({items:e},t,{flowChars:s,itemIndent:a}){const{indent:o,indentStep:l,flowCollectionPadding:c,options:{commentString:f}}=t;a+=l;const m=Object.assign({},t,{indent:a,inFlow:!0,type:null});let p=!1,_=0;const y=[];for(let S=0;S<e.length;++S){const w=e[S];let k=null;if(Rt(w))w.spaceBefore&&y.push(""),fd(t,y,w.commentBefore,!1),w.comment&&(k=w.comment);else if(At(w)){const A=Rt(w.key)?w.key:null;A&&(A.spaceBefore&&y.push(""),fd(t,y,A.commentBefore,!1),A.comment&&(p=!0));const N=Rt(w.value)?w.value:null;N?(N.comment&&(k=N.comment),N.commentBefore&&(p=!0)):w.value==null&&A?.comment&&(k=A.comment)}k&&(p=!0);let j=br(w,m,()=>k=null);p||(p=y.length>_||j.includes(`
`)),S<e.length-1?j+=",":t.options.trailingComma&&(t.options.lineWidth>0&&(p||(p=y.reduce((A,N)=>A+N.length+2,2)+(j.length+2)>t.options.lineWidth)),p&&(j+=",")),k&&(j+=Fa(j,a,f(k))),y.push(j),_=y.length}const{start:b,end:v}=s;if(y.length===0)return b+v;if(!p){const S=y.reduce((w,k)=>w+k.length+2,2);p=t.options.lineWidth>0&&S>t.options.lineWidth}if(p){let S=b;for(const w of y)S+=w?`
${l}${o}${w}`:`
`;return`${S}
${o}${v}`}else return`${b}${c}${y.join(" ")}${c}${v}`}function fd({indent:e,options:{commentString:t}},s,a,o){if(a&&o&&(a=a.replace(/^\n+/,"")),a){const l=Si(t(a),e);s.push(l.trimStart())}}function Ja(e,t){const s=xt(t)?t.value:t;for(const a of e)if(At(a)&&(a.key===t||a.key===s||xt(a.key)&&a.key.value===s))return a}class ns extends Uw{static get tagName(){return"tag:yaml.org,2002:map"}constructor(t){super(ma,t),this.items=[]}static from(t,s,a){const{keepUndefined:o,replacer:l}=a,c=new this(t),f=(m,p)=>{if(typeof l=="function")p=l.call(s,m,p);else if(Array.isArray(l)&&!l.includes(m))return;(p!==void 0||o)&&c.items.push(Ih(m,p,a))};if(s instanceof Map)for(const[m,p]of s)f(m,p);else if(s&&typeof s=="object")for(const m of Object.keys(s))f(m,s[m]);return typeof t.sortMapEntries=="function"&&c.items.sort(t.sortMapEntries),c}add(t,s){let a;At(t)?a=t:!t||typeof t!="object"||!("key"in t)?a=new jn(t,t?.value):a=new jn(t.key,t.value);const o=Ja(this.items,a.key),l=this.schema?.sortMapEntries;if(o){if(!s)throw new Error(`Key ${a.key} already set`);xt(o.value)&&qw(a.value)?o.value.value=a.value:o.value=a.value}else if(l){const c=this.items.findIndex(f=>l(a,f)<0);c===-1?this.items.push(a):this.items.splice(c,0,a)}else this.items.push(a)}delete(t){const s=Ja(this.items,t);return s?this.items.splice(this.items.indexOf(s),1).length>0:!1}get(t,s){const o=Ja(this.items,t)?.value;return(!s&&xt(o)?o.value:o)??void 0}has(t){return!!Ja(this.items,t)}set(t,s){this.add(new jn(t,s),!0)}toJSON(t,s,a){const o=a?new a:s?.mapAsMap?new Map:{};s?.onCreate&&s.onCreate(o);for(const l of this.items)Yw(s,o,l);return o}toString(t,s,a){if(!t)return JSON.stringify(this);for(const o of this.items)if(!At(o))throw new Error(`Map items must all be pairs; found ${JSON.stringify(o)} instead`);return!t.allNullValues&&this.hasAllNullValues(!1)&&(t=Object.assign({},t,{allNullValues:!0})),Vw(this,t,{blockItemPrefix:"",flowChars:{start:"{",end:"}"},itemIndent:t.indent||"",onChompKeep:a,onComment:s})}}const zr={collection:"map",default:!0,nodeClass:ns,tag:"tag:yaml.org,2002:map",resolve(e,t){return Pn(e)||t("Expected a mapping for this tag"),e},createNode:(e,t,s)=>ns.from(e,t,s)};class ha extends Uw{static get tagName(){return"tag:yaml.org,2002:seq"}constructor(t){super(Mr,t),this.items=[]}add(t){this.items.push(t)}delete(t){const s=Au(t);return typeof s!="number"?!1:this.items.splice(s,1).length>0}get(t,s){const a=Au(t);if(typeof a!="number")return;const o=this.items[a];return!s&&xt(o)?o.value:o}has(t){const s=Au(t);return typeof s=="number"&&s<this.items.length}set(t,s){const a=Au(t);if(typeof a!="number")throw new Error(`Expected a valid index, not ${t}.`);const o=this.items[a];xt(o)&&qw(s)?o.value=s:this.items[a]=s}toJSON(t,s){const a=[];s?.onCreate&&s.onCreate(a);let o=0;for(const l of this.items)a.push(ys(l,String(o++),s));return a}toString(t,s,a){return t?Vw(this,t,{blockItemPrefix:"- ",flowChars:{start:"[",end:"]"},itemIndent:(t.indent||"")+"  ",onChompKeep:a,onComment:s}):JSON.stringify(this)}static from(t,s,a){const{replacer:o}=a,l=new this(t);if(s&&Symbol.iterator in Object(s)){let c=0;for(let f of s){if(typeof o=="function"){const m=s instanceof Set?f:String(c++);f=o.call(s,m,f)}l.items.push(Ul(f,void 0,a))}}return l}}function Au(e){let t=xt(e)?e.value:e;return t&&typeof t=="string"&&(t=Number(t)),typeof t=="number"&&Number.isInteger(t)&&t>=0?t:null}const Dr={collection:"seq",default:!0,nodeClass:ha,tag:"tag:yaml.org,2002:seq",resolve(e,t){return $s(e)||t("Expected a sequence for this tag"),e},createNode:(e,t,s)=>ha.from(e,t,s)},qd={identify:e=>typeof e=="string",default:!0,tag:"tag:yaml.org,2002:str",resolve:e=>e,stringify(e,t,s,a){return t=Object.assign({actualString:!0},t),nc(e,t,s,a)}},Ud={identify:e=>e==null,createNode:()=>new Ze(null),default:!0,tag:"tag:yaml.org,2002:null",test:/^(?:~|[Nn]ull|NULL)?$/,resolve:()=>new Ze(null),stringify:({source:e},t)=>typeof e=="string"&&Ud.test.test(e)?e:t.options.nullStr},Zh={identify:e=>typeof e=="boolean",default:!0,tag:"tag:yaml.org,2002:bool",test:/^(?:[Tt]rue|TRUE|[Ff]alse|FALSE)$/,resolve:e=>new Ze(e[0]==="t"||e[0]==="T"),stringify({source:e,value:t},s){if(e&&Zh.test.test(e)){const a=e[0]==="t"||e[0]==="T";if(t===a)return e}return t?s.options.trueStr:s.options.falseStr}};function Rs({format:e,minFractionDigits:t,tag:s,value:a}){if(typeof a=="bigint")return String(a);const o=typeof a=="number"?a:Number(a);if(!isFinite(o))return isNaN(o)?".nan":o<0?"-.inf":".inf";let l=Object.is(a,-0)?"-0":JSON.stringify(a);if(!e&&t&&(!s||s==="tag:yaml.org,2002:float")&&/^-?\d/.test(l)&&!l.includes("e")){let c=l.indexOf(".");c<0&&(c=l.length,l+=".");let f=t-(l.length-c-1);for(;f-- >0;)l+="0"}return l}const Kw={identify:e=>typeof e=="number",default:!0,tag:"tag:yaml.org,2002:float",test:/^(?:[-+]?\.(?:inf|Inf|INF)|\.nan|\.NaN|\.NAN)$/,resolve:e=>e.slice(-3).toLowerCase()==="nan"?NaN:e[0]==="-"?Number.NEGATIVE_INFINITY:Number.POSITIVE_INFINITY,stringify:Rs},Gw={identify:e=>typeof e=="number",default:!0,tag:"tag:yaml.org,2002:float",format:"EXP",test:/^[-+]?(?:\.[0-9]+|[0-9]+(?:\.[0-9]*)?)[eE][-+]?[0-9]+$/,resolve:e=>parseFloat(e),stringify(e){const t=Number(e.value);return isFinite(t)?t.toExponential():Rs(e)}},Xw={identify:e=>typeof e=="number",default:!0,tag:"tag:yaml.org,2002:float",test:/^[-+]?(?:\.[0-9]+|[0-9]+\.[0-9]*)$/,resolve(e){const t=new Ze(parseFloat(e)),s=e.indexOf(".");return s!==-1&&e[e.length-1]==="0"&&(t.minFractionDigits=e.length-s-1),t},stringify:Rs},Bd=e=>typeof e=="bigint"||Number.isInteger(e),Ph=(e,t,s,{intAsBigInt:a})=>a?BigInt(e):parseInt(e.substring(t),s);function Fw(e,t,s){const{value:a}=e;return Bd(a)&&a>=0?s+a.toString(t):Rs(e)}const Jw={identify:e=>Bd(e)&&e>=0,default:!0,tag:"tag:yaml.org,2002:int",format:"OCT",test:/^0o[0-7]+$/,resolve:(e,t,s)=>Ph(e,2,8,s),stringify:e=>Fw(e,8,"0o")},Qw={identify:Bd,default:!0,tag:"tag:yaml.org,2002:int",test:/^[-+]?[0-9]+$/,resolve:(e,t,s)=>Ph(e,0,10,s),stringify:Rs},Ww={identify:e=>Bd(e)&&e>=0,default:!0,tag:"tag:yaml.org,2002:int",format:"HEX",test:/^0x[0-9a-fA-F]+$/,resolve:(e,t,s)=>Ph(e,2,16,s),stringify:e=>Fw(e,16,"0x")},mM=[zr,Dr,qd,Ud,Zh,Jw,Qw,Ww,Kw,Gw,Xw];function a0(e){return typeof e=="bigint"||Number.isInteger(e)}const Ou=({value:e})=>JSON.stringify(e),hM=[{identify:e=>typeof e=="string",default:!0,tag:"tag:yaml.org,2002:str",resolve:e=>e,stringify:Ou},{identify:e=>e==null,createNode:()=>new Ze(null),default:!0,tag:"tag:yaml.org,2002:null",test:/^null$/,resolve:()=>null,stringify:Ou},{identify:e=>typeof e=="boolean",default:!0,tag:"tag:yaml.org,2002:bool",test:/^true$|^false$/,resolve:e=>e==="true",stringify:Ou},{identify:a0,default:!0,tag:"tag:yaml.org,2002:int",test:/^-?(?:0|[1-9][0-9]*)$/,resolve:(e,t,{intAsBigInt:s})=>s?BigInt(e):parseInt(e,10),stringify:({value:e})=>a0(e)?e.toString():JSON.stringify(e)},{identify:e=>typeof e=="number",default:!0,tag:"tag:yaml.org,2002:float",test:/^-?(?:0|[1-9][0-9]*)(?:\.[0-9]*)?(?:[eE][-+]?[0-9]+)?$/,resolve:e=>parseFloat(e),stringify:Ou}],gM={default:!0,tag:"",test:/^/,resolve(e,t){return t(`Unresolved plain scalar ${JSON.stringify(e)}`),e}},yM=[zr,Dr].concat(hM,gM),Yh={identify:e=>e instanceof Uint8Array,default:!1,tag:"tag:yaml.org,2002:binary",resolve(e,t){if(typeof atob=="function"){const s=atob(e.replace(/[\n\r]/g,"")),a=new Uint8Array(s.length);for(let o=0;o<s.length;++o)a[o]=s.charCodeAt(o);return a}else return t("This environment does not support reading binary tags; either Buffer or atob is required"),e},stringify({comment:e,type:t,value:s},a,o,l){if(!s)return"";const c=s;let f;if(typeof btoa=="function"){let m="";for(let p=0;p<c.length;++p)m+=String.fromCharCode(c[p]);f=btoa(m)}else throw new Error("This environment does not support writing binary tags; either Buffer or btoa is required");if(t??(t=Ze.BLOCK_LITERAL),t!==Ze.QUOTE_DOUBLE){const m=Math.max(a.options.lineWidth-a.indent.length,a.options.minContentWidth),p=Math.ceil(f.length/m),_=new Array(p);for(let y=0,b=0;y<p;++y,b+=m)_[y]=f.substr(b,m);f=_.join(t===Ze.BLOCK_LITERAL?`
`:" ")}return nc({comment:e,type:t,value:f},a,o,l)}};function eS(e,t){if($s(e))for(let s=0;s<e.items.length;++s){let a=e.items[s];if(!At(a)){if(Pn(a)){a.items.length>1&&t("Each pair must have its own sequence indicator");const o=a.items[0]||new jn(new Ze(null));if(a.commentBefore&&(o.key.commentBefore=o.key.commentBefore?`${a.commentBefore}
${o.key.commentBefore}`:a.commentBefore),a.comment){const l=o.value??o.key;l.comment=l.comment?`${a.comment}
${l.comment}`:a.comment}a=o}e.items[s]=At(a)?a:new jn(a)}}else t("Expected a sequence for this tag");return e}function tS(e,t,s){const{replacer:a}=s,o=new ha(e);o.tag="tag:yaml.org,2002:pairs";let l=0;if(t&&Symbol.iterator in Object(t))for(let c of t){typeof a=="function"&&(c=a.call(t,String(l++),c));let f,m;if(Array.isArray(c))if(c.length===2)f=c[0],m=c[1];else throw new TypeError(`Expected [key, value] tuple: ${c}`);else if(c&&c instanceof Object){const p=Object.keys(c);if(p.length===1)f=p[0],m=c[f];else throw new TypeError(`Expected tuple with one key, not ${p.length} keys`)}else f=c;o.items.push(Ih(f,m,s))}return o}const Vh={collection:"seq",default:!1,tag:"tag:yaml.org,2002:pairs",resolve:eS,createNode:tS};class fr extends ha{constructor(){super(),this.add=ns.prototype.add.bind(this),this.delete=ns.prototype.delete.bind(this),this.get=ns.prototype.get.bind(this),this.has=ns.prototype.has.bind(this),this.set=ns.prototype.set.bind(this),this.tag=fr.tag}toJSON(t,s){if(!s)return super.toJSON(t);const a=new Map;s?.onCreate&&s.onCreate(a);for(const o of this.items){let l,c;if(At(o)?(l=ys(o.key,"",s),c=ys(o.value,l,s)):l=ys(o,"",s),a.has(l))throw new Error("Ordered maps must not include duplicate keys");a.set(l,c)}return a}static from(t,s,a){const o=tS(t,s,a),l=new this;return l.items=o.items,l}}fr.tag="tag:yaml.org,2002:omap";const Kh={collection:"seq",identify:e=>e instanceof Map,nodeClass:fr,default:!1,tag:"tag:yaml.org,2002:omap",resolve(e,t){const s=eS(e,t),a=[];for(const{key:o}of s.items)xt(o)&&(a.includes(o.value)?t(`Ordered maps must not include duplicate keys: ${o.value}`):a.push(o.value));return Object.assign(new fr,s)},createNode:(e,t,s)=>fr.from(e,t,s)};function nS({value:e,source:t},s){return t&&(e?sS:iS).test.test(t)?t:e?s.options.trueStr:s.options.falseStr}const sS={identify:e=>e===!0,default:!0,tag:"tag:yaml.org,2002:bool",test:/^(?:Y|y|[Yy]es|YES|[Tt]rue|TRUE|[Oo]n|ON)$/,resolve:()=>new Ze(!0),stringify:nS},iS={identify:e=>e===!1,default:!0,tag:"tag:yaml.org,2002:bool",test:/^(?:N|n|[Nn]o|NO|[Ff]alse|FALSE|[Oo]ff|OFF)$/,resolve:()=>new Ze(!1),stringify:nS},_M={identify:e=>typeof e=="number",default:!0,tag:"tag:yaml.org,2002:float",test:/^(?:[-+]?\.(?:inf|Inf|INF)|\.nan|\.NaN|\.NAN)$/,resolve:e=>e.slice(-3).toLowerCase()==="nan"?NaN:e[0]==="-"?Number.NEGATIVE_INFINITY:Number.POSITIVE_INFINITY,stringify:Rs},bM={identify:e=>typeof e=="number",default:!0,tag:"tag:yaml.org,2002:float",format:"EXP",test:/^[-+]?(?:[0-9][0-9_]*)?(?:\.[0-9_]*)?[eE][-+]?[0-9]+$/,resolve:e=>parseFloat(e.replace(/_/g,"")),stringify(e){const t=Number(e.value);return isFinite(t)?t.toExponential():Rs(e)}},vM={identify:e=>typeof e=="number",default:!0,tag:"tag:yaml.org,2002:float",test:/^[-+]?(?:[0-9][0-9_]*)?\.[0-9_]*$/,resolve(e){const t=new Ze(parseFloat(e.replace(/_/g,""))),s=e.indexOf(".");if(s!==-1){const a=e.substring(s+1).replace(/_/g,"");a[a.length-1]==="0"&&(t.minFractionDigits=a.length)}return t},stringify:Rs},sc=e=>typeof e=="bigint"||Number.isInteger(e);function Hd(e,t,s,{intAsBigInt:a}){const o=e[0];if((o==="-"||o==="+")&&(t+=1),e=e.substring(t).replace(/_/g,""),a){switch(s){case 2:e=`0b${e}`;break;case 8:e=`0o${e}`;break;case 16:e=`0x${e}`;break}const c=BigInt(e);return o==="-"?BigInt(-1)*c:c}const l=parseInt(e,s);return o==="-"?-1*l:l}function Gh(e,t,s){const{value:a}=e;if(sc(a)){const o=a.toString(t);return a<0?"-"+s+o.substr(1):s+o}return Rs(e)}const xM={identify:sc,default:!0,tag:"tag:yaml.org,2002:int",format:"BIN",test:/^[-+]?0b[0-1_]+$/,resolve:(e,t,s)=>Hd(e,2,2,s),stringify:e=>Gh(e,2,"0b")},wM={identify:sc,default:!0,tag:"tag:yaml.org,2002:int",format:"OCT",test:/^[-+]?0[0-7_]+$/,resolve:(e,t,s)=>Hd(e,1,8,s),stringify:e=>Gh(e,8,"0")},SM={identify:sc,default:!0,tag:"tag:yaml.org,2002:int",test:/^[-+]?[0-9][0-9_]*$/,resolve:(e,t,s)=>Hd(e,0,10,s),stringify:Rs},kM={identify:sc,default:!0,tag:"tag:yaml.org,2002:int",format:"HEX",test:/^[-+]?0x[0-9a-fA-F_]+$/,resolve:(e,t,s)=>Hd(e,2,16,s),stringify:e=>Gh(e,16,"0x")};class pr extends ns{constructor(t){super(t),this.tag=pr.tag}add(t){let s;At(t)?s=t:t&&typeof t=="object"&&"key"in t&&"value"in t&&t.value===null?s=new jn(t.key,null):s=new jn(t,null),Ja(this.items,s.key)||this.items.push(s)}get(t,s){const a=Ja(this.items,t);return!s&&At(a)?xt(a.key)?a.key.value:a.key:a}set(t,s){if(typeof s!="boolean")throw new Error(`Expected boolean value for set(key, value) in a YAML set, not ${typeof s}`);const a=Ja(this.items,t);a&&!s?this.items.splice(this.items.indexOf(a),1):!a&&s&&this.items.push(new jn(t))}toJSON(t,s){return super.toJSON(t,s,Set)}toString(t,s,a){if(!t)return JSON.stringify(this);if(this.hasAllNullValues(!0))return super.toString(Object.assign({},t,{allNullValues:!0}),s,a);throw new Error("Set items must all have null values")}static from(t,s,a){const{replacer:o}=a,l=new this(t);if(s&&Symbol.iterator in Object(s))for(let c of s)typeof o=="function"&&(c=o.call(s,c,c)),l.items.push(Ih(c,null,a));return l}}pr.tag="tag:yaml.org,2002:set";const Xh={collection:"map",identify:e=>e instanceof Set,nodeClass:pr,default:!1,tag:"tag:yaml.org,2002:set",createNode:(e,t,s)=>pr.from(e,t,s),resolve(e,t){if(Pn(e)){if(e.hasAllNullValues(!0))return Object.assign(new pr,e);t("Set items must all have null values")}else t("Expected a mapping for this tag");return e}};function Fh(e,t){const s=e[0],a=s==="-"||s==="+"?e.substring(1):e,o=c=>t?BigInt(c):Number(c),l=a.replace(/_/g,"").split(":").reduce((c,f)=>c*o(60)+o(f),o(0));return s==="-"?o(-1)*l:l}function aS(e){let{value:t}=e,s=c=>c;if(typeof t=="bigint")s=c=>BigInt(c);else if(isNaN(t)||!isFinite(t))return Rs(e);let a="";t<0&&(a="-",t*=s(-1));const o=s(60),l=[t%o];return t<60?l.unshift(0):(t=(t-l[0])/o,l.unshift(t%o),t>=60&&(t=(t-l[0])/o,l.unshift(t))),a+l.map(c=>String(c).padStart(2,"0")).join(":").replace(/000000\d*$/,"")}const oS={identify:e=>typeof e=="bigint"||Number.isInteger(e),default:!0,tag:"tag:yaml.org,2002:int",format:"TIME",test:/^[-+]?[0-9][0-9_]*(?::[0-5]?[0-9])+$/,resolve:(e,t,{intAsBigInt:s})=>Fh(e,s),stringify:aS},rS={identify:e=>typeof e=="number",default:!0,tag:"tag:yaml.org,2002:float",format:"TIME",test:/^[-+]?[0-9][0-9_]*(?::[0-5]?[0-9])+\.[0-9_]*$/,resolve:e=>Fh(e,!1),stringify:aS},Id={identify:e=>e instanceof Date,default:!0,tag:"tag:yaml.org,2002:timestamp",test:RegExp("^([0-9]{4})-([0-9]{1,2})-([0-9]{1,2})(?:(?:t|T|[ \\t]+)([0-9]{1,2}):([0-9]{1,2}):([0-9]{1,2}(\\.[0-9]+)?)(?:[ \\t]*(Z|[-+][012]?[0-9](?::[0-9]{2})?))?)?$"),resolve(e){const t=e.match(Id.test);if(!t)throw new Error("!!timestamp expects a date, starting with yyyy-mm-dd");const[,s,a,o,l,c,f]=t.map(Number),m=t[7]?Number((t[7]+"00").substr(1,3)):0;let p=Date.UTC(s,a-1,o,l||0,c||0,f||0,m);const _=t[8];if(_&&_!=="Z"){let y=Fh(_,!1);Math.abs(y)<30&&(y*=60),p-=6e4*y}return new Date(p)},stringify:({value:e})=>e?.toISOString().replace(/(T00:00:00)?\.000Z$/,"")??""},o0=[zr,Dr,qd,Ud,sS,iS,xM,wM,SM,kM,_M,bM,vM,Yh,Ei,Kh,Vh,Xh,oS,rS,Id],r0=new Map([["core",mM],["failsafe",[zr,Dr,qd]],["json",yM],["yaml11",o0],["yaml-1.1",o0]]),l0={binary:Yh,bool:Zh,float:Xw,floatExp:Gw,floatNaN:Kw,floatTime:rS,int:Qw,intHex:Ww,intOct:Jw,intTime:oS,map:zr,merge:Ei,null:Ud,omap:Kh,pairs:Vh,seq:Dr,set:Xh,timestamp:Id},jM={"tag:yaml.org,2002:binary":Yh,"tag:yaml.org,2002:merge":Ei,"tag:yaml.org,2002:omap":Kh,"tag:yaml.org,2002:pairs":Vh,"tag:yaml.org,2002:set":Xh,"tag:yaml.org,2002:timestamp":Id};function wm(e,t,s){const a=r0.get(t);if(a&&!e)return s&&!a.includes(Ei)?a.concat(Ei):a.slice();let o=a;if(!o)if(Array.isArray(e))o=[];else{const l=Array.from(r0.keys()).filter(c=>c!=="yaml11").map(c=>JSON.stringify(c)).join(", ");throw new Error(`Unknown schema "${t}"; use one of ${l} or define customTags array`)}if(Array.isArray(e))for(const l of e)o=o.concat(l);else typeof e=="function"&&(o=e(o.slice()));return s&&(o=o.concat(Ei)),o.reduce((l,c)=>{const f=typeof c=="string"?l0[c]:c;if(!f){const m=JSON.stringify(c),p=Object.keys(l0).map(_=>JSON.stringify(_)).join(", ");throw new Error(`Unknown custom tag ${m}; use one of ${p}`)}return l.includes(f)||l.push(f),l},[])}const NM=(e,t)=>e.key<t.key?-1:e.key>t.key?1:0;class Zd{constructor({compat:t,customTags:s,merge:a,resolveKnownTags:o,schema:l,sortMapEntries:c,toStringDefaults:f}){this.compat=Array.isArray(t)?wm(t,"compat"):t?wm(null,t):null,this.name=typeof l=="string"&&l||"core",this.knownTags=o?jM:{},this.tags=wm(s,this.name,a),this.toStringOptions=f??null,Object.defineProperty(this,ma,{value:zr}),Object.defineProperty(this,Js,{value:qd}),Object.defineProperty(this,Mr,{value:Dr}),this.sortMapEntries=typeof c=="function"?c:c===!0?NM:null}clone(){const t=Object.create(Zd.prototype,Object.getOwnPropertyDescriptors(this));return t.tags=this.tags.slice(),t}}function EM(e,t){const s=[];let a=t.directives===!0;if(t.directives!==!1&&e.directives){const m=e.directives.toString(e);m?(s.push(m),a=!0):e.directives.docStart&&(a=!0)}a&&s.push("---");const o=Hw(e,t),{commentString:l}=o.options;if(e.commentBefore){s.length!==1&&s.unshift("");const m=l(e.commentBefore);s.unshift(Si(m,""))}let c=!1,f=null;if(e.contents){if(Rt(e.contents)){if(e.contents.spaceBefore&&a&&s.push(""),e.contents.commentBefore){const _=l(e.contents.commentBefore);s.push(Si(_,""))}o.forceBlockIndent=!!e.comment,f=e.contents.comment}const m=f?void 0:()=>c=!0;let p=br(e.contents,o,()=>f=null,m);f&&(p+=Fa(p,"",l(f))),(p[0]==="|"||p[0]===">")&&s[s.length-1]==="---"?s[s.length-1]=`--- ${p}`:s.push(p)}else s.push(br(e.contents,o));if(e.directives?.docEnd)if(e.comment){const m=l(e.comment);m.includes(`
`)?(s.push("..."),s.push(Si(m,""))):s.push(`... ${m}`)}else s.push("...");else{let m=e.comment;m&&c&&(m=m.replace(/^\n+/,"")),m&&((!c||f)&&s[s.length-1]!==""&&s.push(""),s.push(Si(l(m),"")))}return s.join(`
`)+`
`}class $r{constructor(t,s,a){this.commentBefore=null,this.comment=null,this.errors=[],this.warnings=[],Object.defineProperty(this,_s,{value:th});let o=null;typeof s=="function"||Array.isArray(s)?o=s:a===void 0&&s&&(a=s,s=void 0);const l=Object.assign({intAsBigInt:!1,keepSourceTokens:!1,logLevel:"warn",prettyErrors:!0,strict:!0,stringKeys:!1,uniqueKeys:!0,version:"1.2"},a);this.options=l;let{version:c}=l;a?._directives?(this.directives=a._directives.atDocument(),this.directives.yaml.explicit&&(c=this.directives.yaml.version)):this.directives=new Mn({version:c}),this.setSchema(c,a),this.contents=t===void 0?null:this.createNode(t,o,a)}clone(){const t=Object.create($r.prototype,{[_s]:{value:th}});return t.commentBefore=this.commentBefore,t.comment=this.comment,t.errors=this.errors.slice(),t.warnings=this.warnings.slice(),t.options=Object.assign({},this.options),this.directives&&(t.directives=this.directives.clone()),t.schema=this.schema.clone(),t.contents=Rt(this.contents)?this.contents.clone(t.schema):this.contents,this.range&&(t.range=this.range.slice()),t}add(t){Wo(this.contents)&&this.contents.add(t)}addIn(t,s){Wo(this.contents)&&this.contents.addIn(t,s)}createAlias(t,s){if(!t.anchor){const a=Rw(this);t.anchor=!s||a.has(s)?Lw(s||"a",a):s}return new Dd(t.anchor)}createNode(t,s,a){let o;if(typeof s=="function")t=s.call({"":t},"",t),o=s;else if(Array.isArray(s)){const k=A=>typeof A=="number"||A instanceof String||A instanceof Number,j=s.filter(k).map(String);j.length>0&&(s=s.concat(j)),o=s}else a===void 0&&s&&(a=s,s=void 0);const{aliasDuplicateObjects:l,anchorPrefix:c,flow:f,keepUndefined:m,onTagObj:p,tag:_}=a??{},{onAnchor:y,setAnchors:b,sourceObjects:v}=tM(this,c||"a"),S={aliasDuplicateObjects:l??!0,keepUndefined:m??!1,onAnchor:y,onTagObj:p,replacer:o,schema:this.schema,sourceObjects:v},w=Ul(t,_,S);return f&&$t(w)&&(w.flow=!0),b(),w}createPair(t,s,a={}){const o=this.createNode(t,null,a),l=this.createNode(s,null,a);return new jn(o,l)}delete(t){return Wo(this.contents)?this.contents.delete(t):!1}deleteIn(t){return Cl(t)?this.contents==null?!1:(this.contents=null,!0):Wo(this.contents)?this.contents.deleteIn(t):!1}get(t,s){return $t(this.contents)?this.contents.get(t,s):void 0}getIn(t,s){return Cl(t)?!s&&xt(this.contents)?this.contents.value:this.contents:$t(this.contents)?this.contents.getIn(t,s):void 0}has(t){return $t(this.contents)?this.contents.has(t):!1}hasIn(t){return Cl(t)?this.contents!==void 0:$t(this.contents)?this.contents.hasIn(t):!1}set(t,s){this.contents==null?this.contents=dd(this.schema,[t],s):Wo(this.contents)&&this.contents.set(t,s)}setIn(t,s){Cl(t)?this.contents=s:this.contents==null?this.contents=dd(this.schema,Array.from(t),s):Wo(this.contents)&&this.contents.setIn(t,s)}setSchema(t,s={}){typeof t=="number"&&(t=String(t));let a;switch(t){case"1.1":this.directives?this.directives.yaml.version="1.1":this.directives=new Mn({version:"1.1"}),a={resolveKnownTags:!1,schema:"yaml-1.1"};break;case"1.2":case"next":this.directives?this.directives.yaml.version=t:this.directives=new Mn({version:t}),a={resolveKnownTags:!0,schema:"core"};break;case null:this.directives&&delete this.directives,a=null;break;default:{const o=JSON.stringify(t);throw new Error(`Expected '1.1', '1.2' or null as first argument, but found: ${o}`)}}if(s.schema instanceof Object)this.schema=s.schema;else if(a)this.schema=new Zd(Object.assign(a,s));else throw new Error("With a null YAML version, the { schema: Schema } option is required")}toJS({json:t,jsonArg:s,mapAsMap:a,maxAliasCount:o,onAnchor:l,reviver:c}={}){const f={anchors:new Map,doc:this,keep:!t,mapAsMap:a===!0,mapKeyWarned:!1,maxAliasCount:typeof o=="number"?o:100},m=ys(this.contents,s??"",f);if(typeof l=="function")for(const{count:p,res:_}of f.anchors.values())l(_,p);return typeof c=="function"?rr(c,{"":m},"",m):m}toJSON(t,s){return this.toJS({json:!0,jsonArg:t,mapAsMap:!1,onAnchor:s})}toString(t={}){if(this.errors.length>0)throw new Error("Document with errors cannot be stringified");if("indent"in t&&(!Number.isInteger(t.indent)||Number(t.indent)<=0)){const s=JSON.stringify(t.indent);throw new Error(`"indent" option must be a positive integer, not ${s}`)}return EM(this,t)}}function Wo(e){if($t(e))return!0;throw new Error("Expected a YAML collection as document contents")}class Jh extends Error{constructor(t,s,a,o){super(),this.name=t,this.code=a,this.message=o,this.pos=s}}class Qa extends Jh{constructor(t,s,a){super("YAMLParseError",t,s,a)}}class lS extends Jh{constructor(t,s,a){super("YAMLWarning",t,s,a)}}const pd=(e,t)=>s=>{if(s.pos[0]===-1)return;s.linePos=s.pos.map(f=>t.linePos(f));const{line:a,col:o}=s.linePos[0];s.message+=` at line ${a}, column ${o}`;let l=o-1,c=e.substring(t.lineStarts[a-1],t.lineStarts[a]).replace(/[\n\r]+$/,"");if(l>=60&&c.length>80){const f=Math.min(l-39,c.length-79);c="…"+c.substring(f),l-=f-1}if(c.length>80&&(c=c.substring(0,79)+"…"),a>1&&/^ *$/.test(c.substring(0,l))){let f=e.substring(t.lineStarts[a-2],t.lineStarts[a-1]);f.length>80&&(f=f.substring(0,79)+`…
`),c=f+c}if(/[^ ]/.test(c)){let f=1;const m=s.linePos[1];m?.line===a&&m.col>o&&(f=Math.max(1,Math.min(m.col-o,80-l)));const p=" ".repeat(l)+"^".repeat(f);s.message+=`:

${c}
${p}
`}};function vr(e,{flow:t,indicator:s,next:a,offset:o,onError:l,parentIndent:c,startOnNewline:f}){let m=!1,p=f,_=f,y="",b="",v=!1,S=!1,w=null,k=null,j=null,A=null,N=null,T=null,E=null;for(const q of e)switch(S&&(q.type!=="space"&&q.type!=="newline"&&q.type!=="comma"&&l(q.offset,"MISSING_CHAR","Tags and anchors must be separated from the next token by white space"),S=!1),w&&(p&&q.type!=="comment"&&q.type!=="newline"&&l(w,"TAB_AS_INDENT","Tabs are not allowed as indentation"),w=null),q.type){case"space":!t&&(s!=="doc-start"||a?.type!=="flow-collection")&&q.source.includes("	")&&(w=q),_=!0;break;case"comment":{_||l(q,"MISSING_CHAR","Comments must be separated from other tokens by white space characters");const Z=q.source.substring(1)||" ";y?y+=b+Z:y=Z,b="",p=!1;break}case"newline":p?y?y+=q.source:(!T||s!=="seq-item-ind")&&(m=!0):b+=q.source,p=!0,v=!0,(k||j)&&(A=q),_=!0;break;case"anchor":k&&l(q,"MULTIPLE_ANCHORS","A node can have at most one anchor"),q.source.endsWith(":")&&l(q.offset+q.source.length-1,"BAD_ALIAS","Anchor ending in : is ambiguous",!0),k=q,E??(E=q.offset),p=!1,_=!1,S=!0;break;case"tag":{j&&l(q,"MULTIPLE_TAGS","A node can have at most one tag"),j=q,E??(E=q.offset),p=!1,_=!1,S=!0;break}case s:(k||j)&&l(q,"BAD_PROP_ORDER",`Anchors and tags must be after the ${q.source} indicator`),T&&l(q,"UNEXPECTED_TOKEN",`Unexpected ${q.source} in ${t??"collection"}`),T=q,p=s==="seq-item-ind"||s==="explicit-key-ind",_=!1;break;case"comma":if(t){N&&l(q,"UNEXPECTED_TOKEN",`Unexpected , in ${t}`),N=q,p=!1,_=!1;break}default:l(q,"UNEXPECTED_TOKEN",`Unexpected ${q.type} token`),p=!1,_=!1}const O=e[e.length-1],R=O?O.offset+O.source.length:o;return S&&a&&a.type!=="space"&&a.type!=="newline"&&a.type!=="comma"&&(a.type!=="scalar"||a.source!=="")&&l(a.offset,"MISSING_CHAR","Tags and anchors must be separated from the next token by white space"),w&&(p&&w.indent<=c||a?.type==="block-map"||a?.type==="block-seq")&&l(w,"TAB_AS_INDENT","Tabs are not allowed as indentation"),{comma:N,found:T,spaceBefore:m,comment:y,hasNewline:v,anchor:k,tag:j,newlineAfterProp:A,end:R,start:E??R}}function Bl(e){if(!e)return null;switch(e.type){case"alias":case"scalar":case"double-quoted-scalar":case"single-quoted-scalar":if(e.source.includes(`
`))return!0;if(e.end){for(const t of e.end)if(t.type==="newline")return!0}return!1;case"flow-collection":for(const t of e.items){for(const s of t.start)if(s.type==="newline")return!0;if(t.sep){for(const s of t.sep)if(s.type==="newline")return!0}if(Bl(t.key)||Bl(t.value))return!0}return!1;default:return!0}}function ah(e,t,s){if(t?.type==="flow-collection"){const a=t.end[0];a.indent===e&&(a.source==="]"||a.source==="}")&&Bl(t)&&s(a,"BAD_INDENT","Flow end indicator should be more indented than parent",!0)}}function cS(e,t,s){const{uniqueKeys:a}=e.options;if(a===!1)return!1;const o=typeof a=="function"?a:(l,c)=>l===c||xt(l)&&xt(c)&&l.value===c.value;return t.some(l=>o(l.key,s))}const c0="All mapping items must start at the same column";function TM({composeNode:e,composeEmptyNode:t},s,a,o,l){const c=l?.nodeClass??ns,f=new c(s.schema);s.atRoot&&(s.atRoot=!1);let m=a.offset,p=null;for(const _ of a.items){const{start:y,key:b,sep:v,value:S}=_,w=vr(y,{indicator:"explicit-key-ind",next:b??v?.[0],offset:m,onError:o,parentIndent:a.indent,startOnNewline:!0}),k=!w.found;if(k){if(b&&(b.type==="block-seq"?o(m,"BLOCK_AS_IMPLICIT_KEY","A block sequence may not be used as an implicit map key"):"indent"in b&&b.indent!==a.indent&&o(m,"BAD_INDENT",c0)),!w.anchor&&!w.tag&&!v){p=w.end,w.comment&&(f.comment?f.comment+=`
`+w.comment:f.comment=w.comment);continue}(w.newlineAfterProp||Bl(b))&&o(b??y[y.length-1],"MULTILINE_IMPLICIT_KEY","Implicit keys need to be on a single line")}else w.found?.indent!==a.indent&&o(m,"BAD_INDENT",c0);s.atKey=!0;const j=w.end,A=b?e(s,b,w,o):t(s,j,y,null,w,o);s.schema.compat&&ah(a.indent,b,o),s.atKey=!1,cS(s,f.items,A)&&o(j,"DUPLICATE_KEY","Map keys must be unique");const N=vr(v??[],{indicator:"map-value-ind",next:S,offset:A.range[2],onError:o,parentIndent:a.indent,startOnNewline:!b||b.type==="block-scalar"});if(m=N.end,N.found){k&&(S?.type==="block-map"&&!N.hasNewline&&o(m,"BLOCK_AS_IMPLICIT_KEY","Nested mappings are not allowed in compact mappings"),s.options.strict&&w.start<N.found.offset-1024&&o(A.range,"KEY_OVER_1024_CHARS","The : indicator must be at most 1024 chars after the start of an implicit block mapping key"));const T=S?e(s,S,N,o):t(s,m,v,null,N,o);s.schema.compat&&ah(a.indent,S,o),m=T.range[2];const E=new jn(A,T);s.options.keepSourceTokens&&(E.srcToken=_),f.items.push(E)}else{k&&o(A.range,"MISSING_CHAR","Implicit map keys need to be followed by map values"),N.comment&&(A.comment?A.comment+=`
`+N.comment:A.comment=N.comment);const T=new jn(A);s.options.keepSourceTokens&&(T.srcToken=_),f.items.push(T)}}return p&&p<m&&o(p,"IMPOSSIBLE","Map comment with trailing content"),f.range=[a.offset,m,p??m],f}function CM({composeNode:e,composeEmptyNode:t},s,a,o,l){const c=l?.nodeClass??ha,f=new c(s.schema);s.atRoot&&(s.atRoot=!1),s.atKey&&(s.atKey=!1);let m=a.offset,p=null;for(const{start:_,value:y}of a.items){const b=vr(_,{indicator:"seq-item-ind",next:y,offset:m,onError:o,parentIndent:a.indent,startOnNewline:!0});if(!b.found)if(b.anchor||b.tag||y)y?.type==="block-seq"?o(b.end,"BAD_INDENT","All sequence items must start at the same column"):o(m,"MISSING_CHAR","Sequence item without - indicator");else{p=b.end,b.comment&&(f.comment=b.comment);continue}const v=y?e(s,y,b,o):t(s,b.end,_,null,b,o);s.schema.compat&&ah(a.indent,y,o),m=v.range[2],f.items.push(v)}return f.range=[a.offset,m,p??m],f}function ic(e,t,s,a){let o="";if(e){let l=!1,c="";for(const f of e){const{source:m,type:p}=f;switch(p){case"space":l=!0;break;case"comment":{s&&!l&&a(f,"MISSING_CHAR","Comments must be separated from other tokens by white space characters");const _=m.substring(1)||" ";o?o+=c+_:o=_,c="";break}case"newline":o&&(c+=m),l=!0;break;default:a(f,"UNEXPECTED_TOKEN",`Unexpected ${p} at node end`)}t+=m.length}}return{comment:o,offset:t}}const Sm="Block collections are not allowed within flow collections",km=e=>e&&(e.type==="block-map"||e.type==="block-seq");function AM({composeNode:e,composeEmptyNode:t},s,a,o,l){const c=a.start.source==="{",f=c?"flow map":"flow sequence",m=l?.nodeClass??(c?ns:ha),p=new m(s.schema);p.flow=!0;const _=s.atRoot;_&&(s.atRoot=!1),s.atKey&&(s.atKey=!1);let y=a.offset+a.start.source.length;for(let k=0;k<a.items.length;++k){const j=a.items[k],{start:A,key:N,sep:T,value:E}=j,O=vr(A,{flow:f,indicator:"explicit-key-ind",next:N??T?.[0],offset:y,onError:o,parentIndent:a.indent,startOnNewline:!1});if(!O.found){if(!O.anchor&&!O.tag&&!T&&!E){k===0&&O.comma?o(O.comma,"UNEXPECTED_TOKEN",`Unexpected , in ${f}`):k<a.items.length-1&&o(O.start,"UNEXPECTED_TOKEN",`Unexpected empty item in ${f}`),O.comment&&(p.comment?p.comment+=`
`+O.comment:p.comment=O.comment),y=O.end;continue}!c&&s.options.strict&&Bl(N)&&o(N,"MULTILINE_IMPLICIT_KEY","Implicit keys of flow sequence pairs need to be on a single line")}if(k===0)O.comma&&o(O.comma,"UNEXPECTED_TOKEN",`Unexpected , in ${f}`);else if(O.comma||o(O.start,"MISSING_CHAR",`Missing , between ${f} items`),O.comment){let R="";e:for(const q of A)switch(q.type){case"comma":case"space":break;case"comment":R=q.source.substring(1);break e;default:break e}if(R){let q=p.items[p.items.length-1];At(q)&&(q=q.value??q.key),q.comment?q.comment+=`
`+R:q.comment=R,O.comment=O.comment.substring(R.length+1)}}if(!c&&!T&&!O.found){const R=E?e(s,E,O,o):t(s,O.end,T,null,O,o);p.items.push(R),y=R.range[2],km(E)&&o(R.range,"BLOCK_IN_FLOW",Sm)}else{s.atKey=!0;const R=O.end,q=N?e(s,N,O,o):t(s,R,A,null,O,o);km(N)&&o(q.range,"BLOCK_IN_FLOW",Sm),s.atKey=!1;const Z=vr(T??[],{flow:f,indicator:"map-value-ind",next:E,offset:q.range[2],onError:o,parentIndent:a.indent,startOnNewline:!1});if(Z.found){if(!c&&!O.found&&s.options.strict){if(T)for(const ee of T){if(ee===Z.found)break;if(ee.type==="newline"){o(ee,"MULTILINE_IMPLICIT_KEY","Implicit keys of flow sequence pairs need to be on a single line");break}}O.start<Z.found.offset-1024&&o(Z.found,"KEY_OVER_1024_CHARS","The : indicator must be at most 1024 chars after the start of an implicit flow sequence key")}}else E&&("source"in E&&E.source?.[0]===":"?o(E,"MISSING_CHAR",`Missing space after : in ${f}`):o(Z.start,"MISSING_CHAR",`Missing , or : between ${f} items`));const oe=E?e(s,E,Z,o):Z.found?t(s,Z.end,T,null,Z,o):null;oe?km(E)&&o(oe.range,"BLOCK_IN_FLOW",Sm):Z.comment&&(q.comment?q.comment+=`
`+Z.comment:q.comment=Z.comment);const Q=new jn(q,oe);if(s.options.keepSourceTokens&&(Q.srcToken=j),c){const ee=p;cS(s,ee.items,q)&&o(R,"DUPLICATE_KEY","Map keys must be unique"),ee.items.push(Q)}else{const ee=new ns(s.schema);ee.flow=!0,ee.items.push(Q);const X=(oe??q).range;ee.range=[q.range[0],X[1],X[2]],p.items.push(ee)}y=oe?oe.range[2]:Z.end}}const b=c?"}":"]",[v,...S]=a.end;let w=y;if(v?.source===b)w=v.offset+v.source.length;else{const k=f[0].toUpperCase()+f.substring(1),j=_?`${k} must end with a ${b}`:`${k} in block collection must be sufficiently indented and end with a ${b}`;o(y,_?"MISSING_CHAR":"BAD_INDENT",j),v&&v.source.length!==1&&S.unshift(v)}if(S.length>0){const k=ic(S,w,s.options.strict,o);k.comment&&(p.comment?p.comment+=`
`+k.comment:p.comment=k.comment),p.range=[a.offset,w,k.offset]}else p.range=[a.offset,w,w];return p}function jm(e,t,s,a,o,l){const c=s.type==="block-map"?TM(e,t,s,a,l):s.type==="block-seq"?CM(e,t,s,a,l):AM(e,t,s,a,l),f=c.constructor;return o==="!"||o===f.tagName?(c.tag=f.tagName,c):(o&&(c.tag=o),c)}function OM(e,t,s,a,o){const l=a.tag,c=l?t.directives.tagName(l.source,b=>o(l,"TAG_RESOLVE_FAILED",b)):null;if(s.type==="block-seq"){const{anchor:b,newlineAfterProp:v}=a,S=b&&l?b.offset>l.offset?b:l:b??l;S&&(!v||v.offset<S.offset)&&o(S,"MISSING_CHAR","Missing newline after block sequence props")}const f=s.type==="block-map"?"map":s.type==="block-seq"?"seq":s.start.source==="{"?"map":"seq";if(!l||!c||c==="!"||c===ns.tagName&&f==="map"||c===ha.tagName&&f==="seq")return jm(e,t,s,o,c);let m=t.schema.tags.find(b=>b.tag===c&&b.collection===f);if(!m){const b=t.schema.knownTags[c];if(b?.collection===f)t.schema.tags.push(Object.assign({},b,{default:!1})),m=b;else return b?o(l,"BAD_COLLECTION_TYPE",`${b.tag} used for ${f} collection, but expects ${b.collection??"scalar"}`,!0):o(l,"TAG_RESOLVE_FAILED",`Unresolved tag: ${c}`,!0),jm(e,t,s,o,c)}const p=jm(e,t,s,o,c,m),_=m.resolve?.(p,b=>o(l,"TAG_RESOLVE_FAILED",b),t.options)??p,y=Rt(_)?_:new Ze(_);return y.range=p.range,y.tag=c,m?.format&&(y.format=m.format),y}function uS(e,t,s){const a=t.offset,o=MM(t,e.options.strict,s);if(!o)return{value:"",type:null,comment:"",range:[a,a,a]};const l=o.mode===">"?Ze.BLOCK_FOLDED:Ze.BLOCK_LITERAL,c=t.source?zM(t.source):[];let f=c.length;for(let w=c.length-1;w>=0;--w){const k=c[w][1];if(k===""||k==="\r")f=w;else break}if(f===0){const w=o.chomp==="+"&&c.length>0?`
`.repeat(Math.max(1,c.length-1)):"";let k=a+o.length;return t.source&&(k+=t.source.length),{value:w,type:l,comment:o.comment,range:[a,k,k]}}let m=t.indent+o.indent,p=t.offset+o.length,_=0;for(let w=0;w<f;++w){const[k,j]=c[w];if(j===""||j==="\r")o.indent===0&&k.length>m&&(m=k.length);else{k.length<m&&s(p+k.length,"MISSING_CHAR","Block scalars with more-indented leading empty lines must use an explicit indentation indicator"),o.indent===0&&(m=k.length),_=w,m===0&&!e.atRoot&&s(p,"BAD_INDENT","Block scalar values in collections must be indented");break}p+=k.length+j.length+1}for(let w=c.length-1;w>=f;--w)c[w][0].length>m&&(f=w+1);let y="",b="",v=!1;for(let w=0;w<_;++w)y+=c[w][0].slice(m)+`
`;for(let w=_;w<f;++w){let[k,j]=c[w];p+=k.length+j.length+1;const A=j[j.length-1]==="\r";if(A&&(j=j.slice(0,-1)),j&&k.length<m){const T=`Block scalar lines must not be less indented than their ${o.indent?"explicit indentation indicator":"first line"}`;s(p-j.length-(A?2:1),"BAD_INDENT",T),k=""}l===Ze.BLOCK_LITERAL?(y+=b+k.slice(m)+j,b=`
`):k.length>m||j[0]==="	"?(b===" "?b=`
`:!v&&b===`
`&&(b=`

`),y+=b+k.slice(m)+j,b=`
`,v=!0):j===""?b===`
`?y+=`
`:b=`
`:(y+=b+j,b=" ",v=!1)}switch(o.chomp){case"-":break;case"+":for(let w=f;w<c.length;++w)y+=`
`+c[w][0].slice(m);y[y.length-1]!==`
`&&(y+=`
`);break;default:y+=`
`}const S=a+o.length+t.source.length;return{value:y,type:l,comment:o.comment,range:[a,S,S]}}function MM({offset:e,props:t},s,a){if(t[0].type!=="block-scalar-header")return a(t[0],"IMPOSSIBLE","Block scalar header not found"),null;const{source:o}=t[0],l=o[0];let c=0,f="",m=-1;for(let b=1;b<o.length;++b){const v=o[b];if(!f&&(v==="-"||v==="+"))f=v;else{const S=Number(v);!c&&S?c=S:m===-1&&(m=e+b)}}m!==-1&&a(m,"UNEXPECTED_TOKEN",`Block scalar header includes extra characters: ${o}`);let p=!1,_="",y=o.length;for(let b=1;b<t.length;++b){const v=t[b];switch(v.type){case"space":p=!0;case"newline":y+=v.source.length;break;case"comment":s&&!p&&a(v,"MISSING_CHAR","Comments must be separated from other tokens by white space characters"),y+=v.source.length,_=v.source.substring(1);break;case"error":a(v,"UNEXPECTED_TOKEN",v.message),y+=v.source.length;break;default:{const S=`Unexpected token in block scalar header: ${v.type}`;a(v,"UNEXPECTED_TOKEN",S);const w=v.source;w&&typeof w=="string"&&(y+=w.length)}}}return{mode:l,indent:c,chomp:f,comment:_,length:y}}function zM(e){const t=e.split(/\n( *)/),s=t[0],a=s.match(/^( *)/),l=[a?.[1]?[a[1],s.slice(a[1].length)]:["",s]];for(let c=1;c<t.length;c+=2)l.push([t[c],t[c+1]]);return l}function dS(e,t,s){const{offset:a,type:o,source:l,end:c}=e;let f,m;const p=(b,v,S)=>s(a+b,v,S);switch(o){case"scalar":f=Ze.PLAIN,m=DM(l,p);break;case"single-quoted-scalar":f=Ze.QUOTE_SINGLE,m=$M(l,p);break;case"double-quoted-scalar":f=Ze.QUOTE_DOUBLE,m=RM(l,p);break;default:return s(e,"UNEXPECTED_TOKEN",`Expected a flow scalar value, but found: ${o}`),{value:"",type:null,comment:"",range:[a,a+l.length,a+l.length]}}const _=a+l.length,y=ic(c,_,t,s);return{value:m,type:f,comment:y.comment,range:[a,_,y.offset]}}function DM(e,t){let s="";switch(e[0]){case"	":s="a tab character";break;case",":s="flow indicator character ,";break;case"%":s="directive indicator character %";break;case"|":case">":{s=`block scalar indicator ${e[0]}`;break}case"@":case"`":{s=`reserved character ${e[0]}`;break}}return s&&t(0,"BAD_SCALAR_START",`Plain value cannot start with ${s}`),fS(e)}function $M(e,t){return(e[e.length-1]!=="'"||e.length===1)&&t(e.length,"MISSING_CHAR","Missing closing 'quote"),fS(e.slice(1,-1)).replace(/''/g,"'")}function fS(e){const t=/(.*?)\r?\n/sy;let s=t.exec(e);if(!s)return e;let a,o;try{a=new RegExp("(?<![ 	])[ 	]+$"),o=new RegExp("^[ 	]+|(?<![ 	])[ 	]+$","g")}catch{a=/[ \t]+$/,o=/^[ \t]+|[ \t]+$/g}let l=s[1].replace(a,""),c=" ",f=t.lastIndex;for(;s=t.exec(e);){const p=s[1].replace(o,"");p===""?c===`
`?l+=c:c=`
`:(l+=c+p,c=" "),f=t.lastIndex}const m=/[ \t]*(.*)/sy;return m.lastIndex=f,s=m.exec(e),l+c+(s?.[1]??"")}function RM(e,t){let s="";for(let a=1;a<e.length-1;++a){const o=e[a];if(!(o==="\r"&&e[a+1]===`
`))if(o===`
`){const{fold:l,offset:c}=LM(e,a);s+=l,a=c}else if(o==="\\"){let l=e[++a];const c=qM[l];if(c)s+=c;else if(l===`
`)for(l=e[a+1];l===" "||l==="	";)l=e[++a+1];else if(l==="\r"&&e[a+1]===`
`)for(l=e[++a+1];l===" "||l==="	";)l=e[++a+1];else if(l==="x"||l==="u"||l==="U"){const f=l==="x"?2:l==="u"?4:8;s+=UM(e,a+1,f,t),a+=f}else{const f=e.substr(a-1,2);t(a-1,"BAD_DQ_ESCAPE",`Invalid escape sequence ${f}`),s+=f}}else if(o===" "||o==="	"){const l=a;let c=e[a+1];for(;c===" "||c==="	";)c=e[++a+1];c!==`
`&&!(c==="\r"&&e[a+2]===`
`)&&(s+=a>l?e.slice(l,a+1):o)}else s+=o}return(e[e.length-1]!=='"'||e.length===1)&&t(e.length,"MISSING_CHAR",'Missing closing "quote'),s}function LM(e,t){let s="",a=e[t+1];for(;(a===" "||a==="	"||a===`
`||a==="\r")&&!(a==="\r"&&e[t+2]!==`
`);)a===`
`&&(s+=`
`),t+=1,a=e[t+1];return s||(s=" "),{fold:s,offset:t}}const qM={0:"\0",a:"\x07",b:"\b",e:"\x1B",f:"\f",n:`
`,r:"\r",t:"	",v:"\v",N:"",_:" ",L:"\u2028",P:"\u2029"," ":" ",'"':'"',"/":"/","\\":"\\","	":"	"};function UM(e,t,s,a){const o=e.substr(t,s),c=o.length===s&&/^[0-9a-fA-F]+$/.test(o)?parseInt(o,16):NaN;try{return String.fromCodePoint(c)}catch{const f=e.substr(t-2,s+2);return a(t-2,"BAD_DQ_ESCAPE",`Invalid escape sequence ${f}`),f}}function pS(e,t,s,a){const{value:o,type:l,comment:c,range:f}=t.type==="block-scalar"?uS(e,t,a):dS(t,e.options.strict,a),m=s?e.directives.tagName(s.source,y=>a(s,"TAG_RESOLVE_FAILED",y)):null;let p;e.options.stringKeys&&e.atKey?p=e.schema[Js]:m?p=BM(e.schema,o,m,s,a):t.type==="scalar"?p=HM(e,o,t,a):p=e.schema[Js];let _;try{const y=p.resolve(o,b=>a(s??t,"TAG_RESOLVE_FAILED",b),e.options);_=xt(y)?y:new Ze(y)}catch(y){const b=y instanceof Error?y.message:String(y);a(s??t,"TAG_RESOLVE_FAILED",b),_=new Ze(o)}return _.range=f,_.source=o,l&&(_.type=l),m&&(_.tag=m),p.format&&(_.format=p.format),c&&(_.comment=c),_}function BM(e,t,s,a,o){if(s==="!")return e[Js];const l=[];for(const f of e.tags)if(!f.collection&&f.tag===s)if(f.default&&f.test)l.push(f);else return f;for(const f of l)if(f.test?.test(t))return f;const c=e.knownTags[s];return c&&!c.collection?(e.tags.push(Object.assign({},c,{default:!1,test:void 0})),c):(o(a,"TAG_RESOLVE_FAILED",`Unresolved tag: ${s}`,s!=="tag:yaml.org,2002:str"),e[Js])}function HM({atKey:e,directives:t,schema:s},a,o,l){const c=s.tags.find(f=>(f.default===!0||e&&f.default==="key")&&f.test?.test(a))||s[Js];if(s.compat){const f=s.compat.find(m=>m.default&&m.test?.test(a))??s[Js];if(c.tag!==f.tag){const m=t.tagString(c.tag),p=t.tagString(f.tag),_=`Value may be parsed as either ${m} or ${p}`;l(o,"TAG_RESOLVE_FAILED",_,!0)}}return c}function IM(e,t,s){if(t){s??(s=t.length);for(let a=s-1;a>=0;--a){let o=t[a];switch(o.type){case"space":case"comment":case"newline":e-=o.source.length;continue}for(o=t[++a];o?.type==="space";)e+=o.source.length,o=t[++a];break}}return e}const ZM={composeNode:mS,composeEmptyNode:Qh};function mS(e,t,s,a){const o=e.atKey,{spaceBefore:l,comment:c,anchor:f,tag:m}=s;let p,_=!0;switch(t.type){case"alias":p=PM(e,t,a),(f||m)&&a(t,"ALIAS_PROPS","An alias node must not specify any properties");break;case"scalar":case"single-quoted-scalar":case"double-quoted-scalar":case"block-scalar":p=pS(e,t,m,a),f&&(p.anchor=f.source.substring(1));break;case"block-map":case"block-seq":case"flow-collection":try{p=OM(ZM,e,t,s,a),f&&(p.anchor=f.source.substring(1))}catch(y){const b=y instanceof Error?y.message:String(y);a(t,"RESOURCE_EXHAUSTION",b)}break;default:{const y=t.type==="error"?t.message:`Unsupported token (type: ${t.type})`;a(t,"UNEXPECTED_TOKEN",y),_=!1}}return p??(p=Qh(e,t.offset,void 0,null,s,a)),f&&p.anchor===""&&a(f,"BAD_ALIAS","Anchor cannot be an empty string"),o&&e.options.stringKeys&&(!xt(p)||typeof p.value!="string"||p.tag&&p.tag!=="tag:yaml.org,2002:str")&&a(m??t,"NON_STRING_KEY","With stringKeys, all keys must be strings"),l&&(p.spaceBefore=!0),c&&(t.type==="scalar"&&t.source===""?p.comment=c:p.commentBefore=c),e.options.keepSourceTokens&&_&&(p.srcToken=t),p}function Qh(e,t,s,a,{spaceBefore:o,comment:l,anchor:c,tag:f,end:m},p){const _={type:"scalar",offset:IM(t,s,a),indent:-1,source:""},y=pS(e,_,f,p);return c&&(y.anchor=c.source.substring(1),y.anchor===""&&p(c,"BAD_ALIAS","Anchor cannot be an empty string")),o&&(y.spaceBefore=!0),l&&(y.comment=l,y.range[2]=m),y}function PM({options:e},{offset:t,source:s,end:a},o){const l=new Dd(s.substring(1));l.source===""&&o(t,"BAD_ALIAS","Alias cannot be an empty string"),l.source.endsWith(":")&&o(t+s.length-1,"BAD_ALIAS","Alias ending in : is ambiguous",!0);const c=t+s.length,f=ic(a,c,e.strict,o);return l.range=[t,c,f.offset],f.comment&&(l.comment=f.comment),l}function YM(e,t,{offset:s,start:a,value:o,end:l},c){const f=Object.assign({_directives:t},e),m=new $r(void 0,f),p={atKey:!1,atRoot:!0,directives:m.directives,options:m.options,schema:m.schema},_=vr(a,{indicator:"doc-start",next:o??l?.[0],offset:s,onError:c,parentIndent:0,startOnNewline:!0});_.found&&(m.directives.docStart=!0,o&&(o.type==="block-map"||o.type==="block-seq")&&!_.hasNewline&&c(_.end,"MISSING_CHAR","Block collection cannot start on same line with directives-end marker")),m.contents=o?mS(p,o,_,c):Qh(p,_.end,a,null,_,c);const y=m.contents.range[2],b=ic(l,y,!1,c);return b.comment&&(m.comment=b.comment),m.range=[s,y,b.offset],m}function jl(e){if(typeof e=="number")return[e,e+1];if(Array.isArray(e))return e.length===2?e:[e[0],e[1]];const{offset:t,source:s}=e;return[t,t+(typeof s=="string"?s.length:1)]}function u0(e){let t="",s=!1,a=!1;for(let o=0;o<e.length;++o){const l=e[o];switch(l[0]){case"#":t+=(t===""?"":a?`

`:`
`)+(l.substring(1)||" "),s=!0,a=!1;break;case"%":e[o+1]?.[0]!=="#"&&(o+=1),s=!1;break;default:s||(a=!0),s=!1}}return{comment:t,afterEmptyLine:a}}class Wh{constructor(t={}){this.doc=null,this.atDirectives=!1,this.prelude=[],this.errors=[],this.warnings=[],this.onError=(s,a,o,l)=>{const c=jl(s);l?this.warnings.push(new lS(c,a,o)):this.errors.push(new Qa(c,a,o))},this.directives=new Mn({version:t.version||"1.2"}),this.options=t}decorate(t,s){const{comment:a,afterEmptyLine:o}=u0(this.prelude);if(a){const l=t.contents;if(s)t.comment=t.comment?`${t.comment}
${a}`:a;else if(o||t.directives.docStart||!l)t.commentBefore=a;else if($t(l)&&!l.flow&&l.items.length>0){let c=l.items[0];At(c)&&(c=c.key);const f=c.commentBefore;c.commentBefore=f?`${a}
${f}`:a}else{const c=l.commentBefore;l.commentBefore=c?`${a}
${c}`:a}}if(s){for(let l=0;l<this.errors.length;++l)t.errors.push(this.errors[l]);for(let l=0;l<this.warnings.length;++l)t.warnings.push(this.warnings[l])}else t.errors=this.errors,t.warnings=this.warnings;this.prelude=[],this.errors=[],this.warnings=[]}streamInfo(){return{comment:u0(this.prelude).comment,directives:this.directives,errors:this.errors,warnings:this.warnings}}*compose(t,s=!1,a=-1){for(const o of t)yield*this.next(o);yield*this.end(s,a)}*next(t){switch(t.type){case"directive":this.directives.add(t.source,(s,a,o)=>{const l=jl(t);l[0]+=s,this.onError(l,"BAD_DIRECTIVE",a,o)}),this.prelude.push(t.source),this.atDirectives=!0;break;case"document":{const s=YM(this.options,this.directives,t,this.onError);this.atDirectives&&!s.directives.docStart&&this.onError(t,"MISSING_CHAR","Missing directives-end/doc-start indicator line"),this.decorate(s,!1),this.doc&&(yield this.doc),this.doc=s,this.atDirectives=!1;break}case"byte-order-mark":case"space":break;case"comment":case"newline":this.prelude.push(t.source);break;case"error":{const s=t.source?`${t.message}: ${JSON.stringify(t.source)}`:t.message,a=new Qa(jl(t),"UNEXPECTED_TOKEN",s);this.atDirectives||!this.doc?this.errors.push(a):this.doc.errors.push(a);break}case"doc-end":{if(!this.doc){const a="Unexpected doc-end without preceding document";this.errors.push(new Qa(jl(t),"UNEXPECTED_TOKEN",a));break}this.doc.directives.docEnd=!0;const s=ic(t.end,t.offset+t.source.length,this.doc.options.strict,this.onError);if(this.decorate(this.doc,!0),s.comment){const a=this.doc.comment;this.doc.comment=a?`${a}
${s.comment}`:s.comment}this.doc.range[2]=s.offset;break}default:this.errors.push(new Qa(jl(t),"UNEXPECTED_TOKEN",`Unsupported token ${t.type}`))}}*end(t=!1,s=-1){if(this.doc)this.decorate(this.doc,!0),yield this.doc,this.doc=null;else if(t){const a=Object.assign({_directives:this.directives},this.options),o=new $r(void 0,a);this.atDirectives&&this.onError(s,"MISSING_CHAR","Missing directives-end indicator line"),o.range=[0,s,s],this.decorate(o,!1),yield o}}}function VM(e,t=!0,s){if(e){const a=(o,l,c)=>{const f=typeof o=="number"?o:Array.isArray(o)?o[0]:o.offset;if(s)s(f,l,c);else throw new Qa([f,f+1],l,c)};switch(e.type){case"scalar":case"single-quoted-scalar":case"double-quoted-scalar":return dS(e,t,a);case"block-scalar":return uS({options:{strict:t}},e,a)}}return null}function KM(e,t){const{implicitKey:s=!1,indent:a,inFlow:o=!1,offset:l=-1,type:c="PLAIN"}=t,f=nc({type:c,value:e},{implicitKey:s,indent:a>0?" ".repeat(a):"",inFlow:o,options:{blockQuote:!0,lineWidth:-1}}),m=t.end??[{type:"newline",offset:-1,indent:a,source:`
`}];switch(f[0]){case"|":case">":{const p=f.indexOf(`
`),_=f.substring(0,p),y=f.substring(p+1)+`
`,b=[{type:"block-scalar-header",offset:l,indent:a,source:_}];return hS(b,m)||b.push({type:"newline",offset:-1,indent:a,source:`
`}),{type:"block-scalar",offset:l,indent:a,props:b,source:y}}case'"':return{type:"double-quoted-scalar",offset:l,indent:a,source:f,end:m};case"'":return{type:"single-quoted-scalar",offset:l,indent:a,source:f,end:m};default:return{type:"scalar",offset:l,indent:a,source:f,end:m}}}function GM(e,t,s={}){let{afterKey:a=!1,implicitKey:o=!1,inFlow:l=!1,type:c}=s,f="indent"in e?e.indent:null;if(a&&typeof f=="number"&&(f+=2),!c)switch(e.type){case"single-quoted-scalar":c="QUOTE_SINGLE";break;case"double-quoted-scalar":c="QUOTE_DOUBLE";break;case"block-scalar":{const p=e.props[0];if(p.type!=="block-scalar-header")throw new Error("Invalid block scalar header");c=p.source[0]===">"?"BLOCK_FOLDED":"BLOCK_LITERAL";break}default:c="PLAIN"}const m=nc({type:c,value:t},{implicitKey:o||f===null,indent:f!==null&&f>0?" ".repeat(f):"",inFlow:l,options:{blockQuote:!0,lineWidth:-1}});switch(m[0]){case"|":case">":XM(e,m);break;case'"':Nm(e,m,"double-quoted-scalar");break;case"'":Nm(e,m,"single-quoted-scalar");break;default:Nm(e,m,"scalar")}}function XM(e,t){const s=t.indexOf(`
`),a=t.substring(0,s),o=t.substring(s+1)+`
`;if(e.type==="block-scalar"){const l=e.props[0];if(l.type!=="block-scalar-header")throw new Error("Invalid block scalar header");l.source=a,e.source=o}else{const{offset:l}=e,c="indent"in e?e.indent:-1,f=[{type:"block-scalar-header",offset:l,indent:c,source:a}];hS(f,"end"in e?e.end:void 0)||f.push({type:"newline",offset:-1,indent:c,source:`
`});for(const m of Object.keys(e))m!=="type"&&m!=="offset"&&delete e[m];Object.assign(e,{type:"block-scalar",indent:c,props:f,source:o})}}function hS(e,t){if(t)for(const s of t)switch(s.type){case"space":case"comment":e.push(s);break;case"newline":return e.push(s),!0}return!1}function Nm(e,t,s){switch(e.type){case"scalar":case"double-quoted-scalar":case"single-quoted-scalar":e.type=s,e.source=t;break;case"block-scalar":{const a=e.props.slice(1);let o=t.length;e.props[0].type==="block-scalar-header"&&(o-=e.props[0].source.length);for(const l of a)l.offset+=o;delete e.props,Object.assign(e,{type:s,source:t,end:a});break}case"block-map":case"block-seq":{const o={type:"newline",offset:e.offset+t.length,indent:e.indent,source:`
`};delete e.items,Object.assign(e,{type:s,source:t,end:[o]});break}default:{const a="indent"in e?e.indent:-1,o="end"in e&&Array.isArray(e.end)?e.end.filter(l=>l.type==="space"||l.type==="comment"||l.type==="newline"):[];for(const l of Object.keys(e))l!=="type"&&l!=="offset"&&delete e[l];Object.assign(e,{type:s,indent:a,source:t,end:o})}}}const FM=e=>"type"in e?md(e):Ku(e);function md(e){switch(e.type){case"block-scalar":{let t="";for(const s of e.props)t+=md(s);return t+e.source}case"block-map":case"block-seq":{let t="";for(const s of e.items)t+=Ku(s);return t}case"flow-collection":{let t=e.start.source;for(const s of e.items)t+=Ku(s);for(const s of e.end)t+=s.source;return t}case"document":{let t=Ku(e);if(e.end)for(const s of e.end)t+=s.source;return t}default:{let t=e.source;if("end"in e&&e.end)for(const s of e.end)t+=s.source;return t}}}function Ku({start:e,key:t,sep:s,value:a}){let o="";for(const l of e)o+=l.source;if(t&&(o+=md(t)),s)for(const l of s)o+=l.source;return a&&(o+=md(a)),o}const oh=Symbol("break visit"),JM=Symbol("skip children"),gS=Symbol("remove item");function oo(e,t){"type"in e&&e.type==="document"&&(e={start:e.start,value:e.value}),yS(Object.freeze([]),e,t)}oo.BREAK=oh;oo.SKIP=JM;oo.REMOVE=gS;oo.itemAtPath=(e,t)=>{let s=e;for(const[a,o]of t){const l=s?.[a];if(l&&"items"in l)s=l.items[o];else return}return s};oo.parentCollection=(e,t)=>{const s=oo.itemAtPath(e,t.slice(0,-1)),a=t[t.length-1][0],o=s?.[a];if(o&&"items"in o)return o;throw new Error("Parent collection not found")};function yS(e,t,s){let a=s(t,e);if(typeof a=="symbol")return a;for(const o of["key","value"]){const l=t[o];if(l&&"items"in l){for(let c=0;c<l.items.length;++c){const f=yS(Object.freeze(e.concat([[o,c]])),l.items[c],s);if(typeof f=="number")c=f-1;else{if(f===oh)return oh;f===gS&&(l.items.splice(c,1),c-=1)}}typeof a=="function"&&o==="key"&&(a=a(t,e))}}return typeof a=="function"?a(t,e):a}const Pd="\uFEFF",Yd="",Vd="",Hl="",QM=e=>!!e&&"items"in e,WM=e=>!!e&&(e.type==="scalar"||e.type==="single-quoted-scalar"||e.type==="double-quoted-scalar"||e.type==="block-scalar");function ez(e){switch(e){case Pd:return"<BOM>";case Yd:return"<DOC>";case Vd:return"<FLOW_END>";case Hl:return"<SCALAR>";default:return JSON.stringify(e)}}function _S(e){switch(e){case Pd:return"byte-order-mark";case Yd:return"doc-mode";case Vd:return"flow-error-end";case Hl:return"scalar";case"---":return"doc-start";case"...":return"doc-end";case"":case`
`:case`\r
`:return"newline";case"-":return"seq-item-ind";case"?":return"explicit-key-ind";case":":return"map-value-ind";case"{":return"flow-map-start";case"}":return"flow-map-end";case"[":return"flow-seq-start";case"]":return"flow-seq-end";case",":return"comma"}switch(e[0]){case" ":case"	":return"space";case"#":return"comment";case"%":return"directive-line";case"*":return"alias";case"&":return"anchor";case"!":return"tag";case"'":return"single-quoted-scalar";case'"':return"double-quoted-scalar";case"|":case">":return"block-scalar-header"}return null}const tz=Object.freeze(Object.defineProperty({__proto__:null,BOM:Pd,DOCUMENT:Yd,FLOW_END:Vd,SCALAR:Hl,createScalarToken:KM,isCollection:QM,isScalar:WM,prettyToken:ez,resolveAsScalar:VM,setScalarValue:GM,stringify:FM,tokenType:_S,visit:oo},Symbol.toStringTag,{value:"Module"}));function Ts(e){switch(e){case void 0:case" ":case`
`:case"\r":case"	":return!0;default:return!1}}const d0=new Set("0123456789ABCDEFabcdef"),nz=new Set("0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz-#;/?:@&=+$_.!~*'()"),Mu=new Set(",[]{}"),sz=new Set(` ,[]{}
\r	`),Em=e=>!e||sz.has(e);class bS{constructor(){this.atEnd=!1,this.blockScalarIndent=-1,this.blockScalarKeep=!1,this.buffer="",this.flowKey=!1,this.flowLevel=0,this.indentNext=0,this.indentValue=0,this.lineEndPos=null,this.next=null,this.pos=0}*lex(t,s=!1){if(t){if(typeof t!="string")throw TypeError("source is not a string");this.buffer=this.buffer?this.buffer+t:t,this.lineEndPos=null}this.atEnd=!s;let a=this.next??"stream";for(;a&&(s||this.hasChars(1));)a=yield*this.parseNext(a)}atLineEnd(){let t=this.pos,s=this.buffer[t];for(;s===" "||s==="	";)s=this.buffer[++t];return!s||s==="#"||s===`
`?!0:s==="\r"?this.buffer[t+1]===`
`:!1}charAt(t){return this.buffer[this.pos+t]}continueScalar(t){let s=this.buffer[t];if(this.indentNext>0){let a=0;for(;s===" ";)s=this.buffer[++a+t];if(s==="\r"){const o=this.buffer[a+t+1];if(o===`
`||!o&&!this.atEnd)return t+a+1}return s===`
`||a>=this.indentNext||!s&&!this.atEnd?t+a:-1}if(s==="-"||s==="."){const a=this.buffer.substr(t,3);if((a==="---"||a==="...")&&Ts(this.buffer[t+3]))return-1}return t}getLine(){let t=this.lineEndPos;return(typeof t!="number"||t!==-1&&t<this.pos)&&(t=this.buffer.indexOf(`
`,this.pos),this.lineEndPos=t),t===-1?this.atEnd?this.buffer.substring(this.pos):null:(this.buffer[t-1]==="\r"&&(t-=1),this.buffer.substring(this.pos,t))}hasChars(t){return this.pos+t<=this.buffer.length}setNext(t){return this.buffer=this.buffer.substring(this.pos),this.pos=0,this.lineEndPos=null,this.next=t,null}peek(t){return this.buffer.substr(this.pos,t)}*parseNext(t){switch(t){case"stream":return yield*this.parseStream();case"line-start":return yield*this.parseLineStart();case"block-start":return yield*this.parseBlockStart();case"doc":return yield*this.parseDocument();case"flow":return yield*this.parseFlowCollection();case"quoted-scalar":return yield*this.parseQuotedScalar();case"block-scalar":return yield*this.parseBlockScalar();case"plain-scalar":return yield*this.parsePlainScalar()}}*parseStream(){let t=this.getLine();if(t===null)return this.setNext("stream");if(t[0]===Pd&&(yield*this.pushCount(1),t=t.substring(1)),t[0]==="%"){let s=t.length,a=t.indexOf("#");for(;a!==-1;){const l=t[a-1];if(l===" "||l==="	"){s=a-1;break}else a=t.indexOf("#",a+1)}for(;;){const l=t[s-1];if(l===" "||l==="	")s-=1;else break}const o=(yield*this.pushCount(s))+(yield*this.pushSpaces(!0));return yield*this.pushCount(t.length-o),this.pushNewline(),"stream"}if(this.atLineEnd()){const s=yield*this.pushSpaces(!0);return yield*this.pushCount(t.length-s),yield*this.pushNewline(),"stream"}return yield Yd,yield*this.parseLineStart()}*parseLineStart(){const t=this.charAt(0);if(!t&&!this.atEnd)return this.setNext("line-start");if(t==="-"||t==="."){if(!this.atEnd&&!this.hasChars(4))return this.setNext("line-start");const s=this.peek(3);if((s==="---"||s==="...")&&Ts(this.charAt(3)))return yield*this.pushCount(3),this.indentValue=0,this.indentNext=0,s==="---"?"doc":"stream"}return this.indentValue=yield*this.pushSpaces(!1),this.indentNext>this.indentValue&&!Ts(this.charAt(1))&&(this.indentNext=this.indentValue),yield*this.parseBlockStart()}*parseBlockStart(){const[t,s]=this.peek(2);if(!s&&!this.atEnd)return this.setNext("block-start");if((t==="-"||t==="?"||t===":")&&Ts(s)){const a=(yield*this.pushCount(1))+(yield*this.pushSpaces(!0));return this.indentNext=this.indentValue+1,this.indentValue+=a,"block-start"}return"doc"}*parseDocument(){yield*this.pushSpaces(!0);const t=this.getLine();if(t===null)return this.setNext("doc");let s=yield*this.pushIndicators();switch(t[s]){case"#":yield*this.pushCount(t.length-s);case void 0:return yield*this.pushNewline(),yield*this.parseLineStart();case"{":case"[":return yield*this.pushCount(1),this.flowKey=!1,this.flowLevel=1,"flow";case"}":case"]":return yield*this.pushCount(1),"doc";case"*":return yield*this.pushUntil(Em),"doc";case'"':case"'":return yield*this.parseQuotedScalar();case"|":case">":return s+=yield*this.parseBlockScalarHeader(),s+=yield*this.pushSpaces(!0),yield*this.pushCount(t.length-s),yield*this.pushNewline(),yield*this.parseBlockScalar();default:return yield*this.parsePlainScalar()}}*parseFlowCollection(){let t,s,a=-1;do t=yield*this.pushNewline(),t>0?(s=yield*this.pushSpaces(!1),this.indentValue=a=s):s=0,s+=yield*this.pushSpaces(!0);while(t+s>0);const o=this.getLine();if(o===null)return this.setNext("flow");if((a!==-1&&a<this.indentNext&&o[0]!=="#"||a===0&&(o.startsWith("---")||o.startsWith("..."))&&Ts(o[3]))&&!(a===this.indentNext-1&&this.flowLevel===1&&(o[0]==="]"||o[0]==="}")))return this.flowLevel=0,yield Vd,yield*this.parseLineStart();let l=0;for(;o[l]===",";)l+=yield*this.pushCount(1),l+=yield*this.pushSpaces(!0),this.flowKey=!1;switch(l+=yield*this.pushIndicators(),o[l]){case void 0:return"flow";case"#":return yield*this.pushCount(o.length-l),"flow";case"{":case"[":return yield*this.pushCount(1),this.flowKey=!1,this.flowLevel+=1,"flow";case"}":case"]":return yield*this.pushCount(1),this.flowKey=!0,this.flowLevel-=1,this.flowLevel?"flow":"doc";case"*":return yield*this.pushUntil(Em),"flow";case'"':case"'":return this.flowKey=!0,yield*this.parseQuotedScalar();case":":{const c=this.charAt(1);if(this.flowKey||Ts(c)||c===",")return this.flowKey=!1,yield*this.pushCount(1),yield*this.pushSpaces(!0),"flow"}default:return this.flowKey=!1,yield*this.parsePlainScalar()}}*parseQuotedScalar(){const t=this.charAt(0);let s=this.buffer.indexOf(t,this.pos+1);if(t==="'")for(;s!==-1&&this.buffer[s+1]==="'";)s=this.buffer.indexOf("'",s+2);else for(;s!==-1;){let l=0;for(;this.buffer[s-1-l]==="\\";)l+=1;if(l%2===0)break;s=this.buffer.indexOf('"',s+1)}const a=this.buffer.substring(0,s);let o=a.indexOf(`
`,this.pos);if(o!==-1){for(;o!==-1;){const l=this.continueScalar(o+1);if(l===-1)break;o=a.indexOf(`
`,l)}o!==-1&&(s=o-(a[o-1]==="\r"?2:1))}if(s===-1){if(!this.atEnd)return this.setNext("quoted-scalar");s=this.buffer.length}return yield*this.pushToIndex(s+1,!1),this.flowLevel?"flow":"doc"}*parseBlockScalarHeader(){this.blockScalarIndent=-1,this.blockScalarKeep=!1;let t=this.pos;for(;;){const s=this.buffer[++t];if(s==="+")this.blockScalarKeep=!0;else if(s>"0"&&s<="9")this.blockScalarIndent=Number(s)-1;else if(s!=="-")break}return yield*this.pushUntil(s=>Ts(s)||s==="#")}*parseBlockScalar(){let t=this.pos-1,s=0,a;e:for(let l=this.pos;a=this.buffer[l];++l)switch(a){case" ":s+=1;break;case`
`:t=l,s=0;break;case"\r":{const c=this.buffer[l+1];if(!c&&!this.atEnd)return this.setNext("block-scalar");if(c===`
`)break}default:break e}if(!a&&!this.atEnd)return this.setNext("block-scalar");if(s>=this.indentNext){this.blockScalarIndent===-1?this.indentNext=s:this.indentNext=this.blockScalarIndent+(this.indentNext===0?1:this.indentNext);do{const l=this.continueScalar(t+1);if(l===-1)break;t=this.buffer.indexOf(`
`,l)}while(t!==-1);if(t===-1){if(!this.atEnd)return this.setNext("block-scalar");t=this.buffer.length}}let o=t+1;for(a=this.buffer[o];a===" ";)a=this.buffer[++o];if(a==="	"){for(;a==="	"||a===" "||a==="\r"||a===`
`;)a=this.buffer[++o];t=o-1}else if(!this.blockScalarKeep)do{let l=t-1,c=this.buffer[l];c==="\r"&&(c=this.buffer[--l]);const f=l;for(;c===" ";)c=this.buffer[--l];if(c===`
`&&l>=this.pos&&l+1+s>f)t=l;else break}while(!0);return yield Hl,yield*this.pushToIndex(t+1,!0),yield*this.parseLineStart()}*parsePlainScalar(){const t=this.flowLevel>0;let s=this.pos-1,a=this.pos-1,o;for(;o=this.buffer[++a];)if(o===":"){const l=this.buffer[a+1];if(Ts(l)||t&&Mu.has(l))break;s=a}else if(Ts(o)){let l=this.buffer[a+1];if(o==="\r"&&(l===`
`?(a+=1,o=`
`,l=this.buffer[a+1]):s=a),l==="#"||t&&Mu.has(l))break;if(o===`
`){const c=this.continueScalar(a+1);if(c===-1)break;a=Math.max(a,c-2)}}else{if(t&&Mu.has(o))break;s=a}return!o&&!this.atEnd?this.setNext("plain-scalar"):(yield Hl,yield*this.pushToIndex(s+1,!0),t?"flow":"doc")}*pushCount(t){return t>0?(yield this.buffer.substr(this.pos,t),this.pos+=t,t):0}*pushToIndex(t,s){const a=this.buffer.slice(this.pos,t);return a?(yield a,this.pos+=a.length,a.length):(s&&(yield""),0)}*pushIndicators(){let t=0;e:for(;;){switch(this.charAt(0)){case"!":t+=yield*this.pushTag(),t+=yield*this.pushSpaces(!0);continue e;case"&":t+=yield*this.pushUntil(Em),t+=yield*this.pushSpaces(!0);continue e;case"-":case"?":case":":{const s=this.flowLevel>0,a=this.charAt(1);if(Ts(a)||s&&Mu.has(a)){s?this.flowKey&&(this.flowKey=!1):this.indentNext=this.indentValue+1,t+=yield*this.pushCount(1),t+=yield*this.pushSpaces(!0);continue e}}}break e}return t}*pushTag(){if(this.charAt(1)==="<"){let t=this.pos+2,s=this.buffer[t];for(;!Ts(s)&&s!==">";)s=this.buffer[++t];return yield*this.pushToIndex(s===">"?t+1:t,!1)}else{let t=this.pos+1,s=this.buffer[t];for(;s;)if(nz.has(s))s=this.buffer[++t];else if(s==="%"&&d0.has(this.buffer[t+1])&&d0.has(this.buffer[t+2]))s=this.buffer[t+=3];else break;return yield*this.pushToIndex(t,!1)}}*pushNewline(){const t=this.buffer[this.pos];return t===`
`?yield*this.pushCount(1):t==="\r"&&this.charAt(1)===`
`?yield*this.pushCount(2):0}*pushSpaces(t){let s=this.pos-1,a;do a=this.buffer[++s];while(a===" "||t&&a==="	");const o=s-this.pos;return o>0&&(yield this.buffer.substr(this.pos,o),this.pos=s),o}*pushUntil(t){let s=this.pos,a=this.buffer[s];for(;!t(a);)a=this.buffer[++s];return yield*this.pushToIndex(s,!1)}}class vS{constructor(){this.lineStarts=[],this.addNewLine=t=>this.lineStarts.push(t),this.linePos=t=>{let s=0,a=this.lineStarts.length;for(;s<a;){const l=s+a>>1;this.lineStarts[l]<t?s=l+1:a=l}if(this.lineStarts[s]===t)return{line:s+1,col:1};if(s===0)return{line:0,col:t};const o=this.lineStarts[s-1];return{line:s,col:t-o+1}}}}function da(e,t){for(let s=0;s<e.length;++s)if(e[s].type===t)return!0;return!1}function f0(e){for(let t=0;t<e.length;++t)switch(e[t].type){case"space":case"comment":case"newline":break;default:return t}return-1}function xS(e){switch(e?.type){case"alias":case"scalar":case"single-quoted-scalar":case"double-quoted-scalar":case"flow-collection":return!0;default:return!1}}function zu(e){switch(e.type){case"document":return e.start;case"block-map":{const t=e.items[e.items.length-1];return t.sep??t.start}case"block-seq":return e.items[e.items.length-1].start;default:return[]}}function er(e){if(e.length===0)return[];let t=e.length;e:for(;--t>=0;)switch(e[t].type){case"doc-start":case"explicit-key-ind":case"map-value-ind":case"seq-item-ind":case"newline":break e}for(;e[++t]?.type==="space";);return e.splice(t,e.length)}function hd(e,t){if(t.length<1e5)Array.prototype.push.apply(e,t);else for(let s=0;s<t.length;++s)e.push(t[s])}function p0(e){if(e.start.type==="flow-seq-start")for(const t of e.items)t.sep&&!t.value&&!da(t.start,"explicit-key-ind")&&!da(t.sep,"map-value-ind")&&(t.key&&(t.value=t.key),delete t.key,xS(t.value)?t.value.end?hd(t.value.end,t.sep):t.value.end=t.sep:hd(t.start,t.sep),delete t.sep)}class eg{constructor(t){this.atNewLine=!0,this.atScalar=!1,this.indent=0,this.offset=0,this.onKeyLine=!1,this.stack=[],this.source="",this.type="",this.lexer=new bS,this.onNewLine=t}*parse(t,s=!1){this.onNewLine&&this.offset===0&&this.onNewLine(0);for(const a of this.lexer.lex(t,s))yield*this.next(a);s||(yield*this.end())}*next(t){if(this.source=t,this.atScalar){this.atScalar=!1,yield*this.step(),this.offset+=t.length;return}const s=_S(t);if(s)if(s==="scalar")this.atNewLine=!1,this.atScalar=!0,this.type="scalar";else{switch(this.type=s,yield*this.step(),s){case"newline":this.atNewLine=!0,this.indent=0,this.onNewLine&&this.onNewLine(this.offset+t.length);break;case"space":this.atNewLine&&t[0]===" "&&(this.indent+=t.length);break;case"explicit-key-ind":case"map-value-ind":case"seq-item-ind":this.atNewLine&&(this.indent+=t.length);break;case"doc-mode":case"flow-error-end":return;default:this.atNewLine=!1}this.offset+=t.length}else{const a=`Not a YAML token: ${t}`;yield*this.pop({type:"error",offset:this.offset,message:a,source:t}),this.offset+=t.length}}*end(){for(;this.stack.length>0;)yield*this.pop()}get sourceToken(){return{type:this.type,offset:this.offset,indent:this.indent,source:this.source}}*step(){const t=this.peek(1);if(this.type==="doc-end"&&t?.type!=="doc-end"){for(;this.stack.length>0;)yield*this.pop();this.stack.push({type:"doc-end",offset:this.offset,source:this.source});return}if(!t)return yield*this.stream();switch(t.type){case"document":return yield*this.document(t);case"alias":case"scalar":case"single-quoted-scalar":case"double-quoted-scalar":return yield*this.scalar(t);case"block-scalar":return yield*this.blockScalar(t);case"block-map":return yield*this.blockMap(t);case"block-seq":return yield*this.blockSequence(t);case"flow-collection":return yield*this.flowCollection(t);case"doc-end":return yield*this.documentEnd(t)}yield*this.pop()}peek(t){return this.stack[this.stack.length-t]}*pop(t){const s=t??this.stack.pop();if(!s)yield{type:"error",offset:this.offset,source:"",message:"Tried to pop an empty stack"};else if(this.stack.length===0)yield s;else{const a=this.peek(1);switch(s.type==="block-scalar"?s.indent="indent"in a?a.indent:0:s.type==="flow-collection"&&a.type==="document"&&(s.indent=0),s.type==="flow-collection"&&p0(s),a.type){case"document":a.value=s;break;case"block-scalar":a.props.push(s);break;case"block-map":{const o=a.items[a.items.length-1];if(o.value){a.items.push({start:[],key:s,sep:[]}),this.onKeyLine=!0;return}else if(o.sep)o.value=s;else{Object.assign(o,{key:s,sep:[]}),this.onKeyLine=!o.explicitKey;return}break}case"block-seq":{const o=a.items[a.items.length-1];o.value?a.items.push({start:[],value:s}):o.value=s;break}case"flow-collection":{const o=a.items[a.items.length-1];!o||o.value?a.items.push({start:[],key:s,sep:[]}):o.sep?o.value=s:Object.assign(o,{key:s,sep:[]});return}default:yield*this.pop(),yield*this.pop(s)}if((a.type==="document"||a.type==="block-map"||a.type==="block-seq")&&(s.type==="block-map"||s.type==="block-seq")){const o=s.items[s.items.length-1];o&&!o.sep&&!o.value&&o.start.length>0&&f0(o.start)===-1&&(s.indent===0||o.start.every(l=>l.type!=="comment"||l.indent<s.indent))&&(a.type==="document"?a.end=o.start:a.items.push({start:o.start}),s.items.splice(-1,1))}}}*stream(){switch(this.type){case"directive-line":yield{type:"directive",offset:this.offset,source:this.source};return;case"byte-order-mark":case"space":case"comment":case"newline":yield this.sourceToken;return;case"doc-mode":case"doc-start":{const t={type:"document",offset:this.offset,start:[]};this.type==="doc-start"&&t.start.push(this.sourceToken),this.stack.push(t);return}}yield{type:"error",offset:this.offset,message:`Unexpected ${this.type} token in YAML stream`,source:this.source}}*document(t){if(t.value)return yield*this.lineEnd(t);switch(this.type){case"doc-start":{f0(t.start)!==-1?(yield*this.pop(),yield*this.step()):t.start.push(this.sourceToken);return}case"anchor":case"tag":case"space":case"comment":case"newline":t.start.push(this.sourceToken);return}const s=this.startBlockValue(t);s?this.stack.push(s):yield{type:"error",offset:this.offset,message:`Unexpected ${this.type} token in YAML document`,source:this.source}}*scalar(t){if(this.type==="map-value-ind"){const s=zu(this.peek(2)),a=er(s);let o;t.end?(o=t.end,o.push(this.sourceToken),delete t.end):o=[this.sourceToken];const l={type:"block-map",offset:t.offset,indent:t.indent,items:[{start:a,key:t,sep:o}]};this.onKeyLine=!0,this.stack[this.stack.length-1]=l}else yield*this.lineEnd(t)}*blockScalar(t){switch(this.type){case"space":case"comment":case"newline":t.props.push(this.sourceToken);return;case"scalar":if(t.source=this.source,this.atNewLine=!0,this.indent=0,this.onNewLine){let s=this.source.indexOf(`
`)+1;for(;s!==0;)this.onNewLine(this.offset+s),s=this.source.indexOf(`
`,s)+1}yield*this.pop();break;default:yield*this.pop(),yield*this.step()}}*blockMap(t){const s=t.items[t.items.length-1];switch(this.type){case"newline":if(this.onKeyLine=!1,s.value){const a="end"in s.value?s.value.end:void 0;(Array.isArray(a)?a[a.length-1]:void 0)?.type==="comment"?a?.push(this.sourceToken):t.items.push({start:[this.sourceToken]})}else s.sep?s.sep.push(this.sourceToken):s.start.push(this.sourceToken);return;case"space":case"comment":if(s.value)t.items.push({start:[this.sourceToken]});else if(s.sep)s.sep.push(this.sourceToken);else{if(this.atIndentedComment(s.start,t.indent)){const o=t.items[t.items.length-2]?.value?.end;if(Array.isArray(o)){hd(o,s.start),o.push(this.sourceToken),t.items.pop();return}}s.start.push(this.sourceToken)}return}if(this.indent>=t.indent){const a=!this.onKeyLine&&this.indent===t.indent,o=a&&(s.sep||s.explicitKey)&&this.type!=="seq-item-ind";let l=[];if(o&&s.sep&&!s.value){const c=[];for(let f=0;f<s.sep.length;++f){const m=s.sep[f];switch(m.type){case"newline":c.push(f);break;case"space":break;case"comment":m.indent>t.indent&&(c.length=0);break;default:c.length=0}}c.length>=2&&(l=s.sep.splice(c[1]))}switch(this.type){case"anchor":case"tag":o||s.value?(l.push(this.sourceToken),t.items.push({start:l}),this.onKeyLine=!0):s.sep?s.sep.push(this.sourceToken):s.start.push(this.sourceToken);return;case"explicit-key-ind":!s.sep&&!s.explicitKey?(s.start.push(this.sourceToken),s.explicitKey=!0):o||s.value?(l.push(this.sourceToken),t.items.push({start:l,explicitKey:!0})):this.stack.push({type:"block-map",offset:this.offset,indent:this.indent,items:[{start:[this.sourceToken],explicitKey:!0}]}),this.onKeyLine=!0;return;case"map-value-ind":if(s.explicitKey)if(s.sep)if(s.value)t.items.push({start:[],key:null,sep:[this.sourceToken]});else if(da(s.sep,"map-value-ind"))this.stack.push({type:"block-map",offset:this.offset,indent:this.indent,items:[{start:l,key:null,sep:[this.sourceToken]}]});else if(xS(s.key)&&!da(s.sep,"newline")){const c=er(s.start),f=s.key,m=s.sep;m.push(this.sourceToken),delete s.key,delete s.sep,this.stack.push({type:"block-map",offset:this.offset,indent:this.indent,items:[{start:c,key:f,sep:m}]})}else l.length>0?s.sep=s.sep.concat(l,this.sourceToken):s.sep.push(this.sourceToken);else if(da(s.start,"newline"))Object.assign(s,{key:null,sep:[this.sourceToken]});else{const c=er(s.start);this.stack.push({type:"block-map",offset:this.offset,indent:this.indent,items:[{start:c,key:null,sep:[this.sourceToken]}]})}else s.sep?s.value||o?t.items.push({start:l,key:null,sep:[this.sourceToken]}):da(s.sep,"map-value-ind")?this.stack.push({type:"block-map",offset:this.offset,indent:this.indent,items:[{start:[],key:null,sep:[this.sourceToken]}]}):s.sep.push(this.sourceToken):Object.assign(s,{key:null,sep:[this.sourceToken]});this.onKeyLine=!0;return;case"alias":case"scalar":case"single-quoted-scalar":case"double-quoted-scalar":{const c=this.flowScalar(this.type);o||s.value?(t.items.push({start:l,key:c,sep:[]}),this.onKeyLine=!0):s.sep?this.stack.push(c):(Object.assign(s,{key:c,sep:[]}),this.onKeyLine=!0);return}default:{const c=this.startBlockValue(t);if(c){if(c.type==="block-seq"){if(!s.explicitKey&&s.sep&&!da(s.sep,"newline")){yield*this.pop({type:"error",offset:this.offset,message:"Unexpected block-seq-ind on same line with key",source:this.source});return}}else a&&t.items.push({start:l});this.stack.push(c);return}}}}yield*this.pop(),yield*this.step()}*blockSequence(t){const s=t.items[t.items.length-1];switch(this.type){case"newline":if(s.value){const a="end"in s.value?s.value.end:void 0;(Array.isArray(a)?a[a.length-1]:void 0)?.type==="comment"?a?.push(this.sourceToken):t.items.push({start:[this.sourceToken]})}else s.start.push(this.sourceToken);return;case"space":case"comment":if(s.value)t.items.push({start:[this.sourceToken]});else{if(this.atIndentedComment(s.start,t.indent)){const o=t.items[t.items.length-2]?.value?.end;if(Array.isArray(o)){hd(o,s.start),o.push(this.sourceToken),t.items.pop();return}}s.start.push(this.sourceToken)}return;case"anchor":case"tag":if(s.value||this.indent<=t.indent)break;s.start.push(this.sourceToken);return;case"seq-item-ind":if(this.indent!==t.indent)break;s.value||da(s.start,"seq-item-ind")?t.items.push({start:[this.sourceToken]}):s.start.push(this.sourceToken);return}if(this.indent>t.indent){const a=this.startBlockValue(t);if(a){this.stack.push(a);return}}yield*this.pop(),yield*this.step()}*flowCollection(t){const s=t.items[t.items.length-1];if(this.type==="flow-error-end"){let a;do yield*this.pop(),a=this.peek(1);while(a?.type==="flow-collection")}else if(t.end.length===0){switch(this.type){case"comma":case"explicit-key-ind":!s||s.sep?t.items.push({start:[this.sourceToken]}):s.start.push(this.sourceToken);return;case"map-value-ind":!s||s.value?t.items.push({start:[],key:null,sep:[this.sourceToken]}):s.sep?s.sep.push(this.sourceToken):Object.assign(s,{key:null,sep:[this.sourceToken]});return;case"space":case"comment":case"newline":case"anchor":case"tag":!s||s.value?t.items.push({start:[this.sourceToken]}):s.sep?s.sep.push(this.sourceToken):s.start.push(this.sourceToken);return;case"alias":case"scalar":case"single-quoted-scalar":case"double-quoted-scalar":{const o=this.flowScalar(this.type);!s||s.value?t.items.push({start:[],key:o,sep:[]}):s.sep?this.stack.push(o):Object.assign(s,{key:o,sep:[]});return}case"flow-map-end":case"flow-seq-end":t.end.push(this.sourceToken);return}const a=this.startBlockValue(t);a?this.stack.push(a):(yield*this.pop(),yield*this.step())}else{const a=this.peek(2);if(a.type==="block-map"&&(this.type==="map-value-ind"&&a.indent===t.indent||this.type==="newline"&&!a.items[a.items.length-1].sep))yield*this.pop(),yield*this.step();else if(this.type==="map-value-ind"&&a.type!=="flow-collection"){const o=zu(a),l=er(o);p0(t);const c=t.end.splice(1,t.end.length);c.push(this.sourceToken);const f={type:"block-map",offset:t.offset,indent:t.indent,items:[{start:l,key:t,sep:c}]};this.onKeyLine=!0,this.stack[this.stack.length-1]=f}else yield*this.lineEnd(t)}}flowScalar(t){if(this.onNewLine){let s=this.source.indexOf(`
`)+1;for(;s!==0;)this.onNewLine(this.offset+s),s=this.source.indexOf(`
`,s)+1}return{type:t,offset:this.offset,indent:this.indent,source:this.source}}startBlockValue(t){switch(this.type){case"alias":case"scalar":case"single-quoted-scalar":case"double-quoted-scalar":return this.flowScalar(this.type);case"block-scalar-header":return{type:"block-scalar",offset:this.offset,indent:this.indent,props:[this.sourceToken],source:""};case"flow-map-start":case"flow-seq-start":return{type:"flow-collection",offset:this.offset,indent:this.indent,start:this.sourceToken,items:[],end:[]};case"seq-item-ind":return{type:"block-seq",offset:this.offset,indent:this.indent,items:[{start:[this.sourceToken]}]};case"explicit-key-ind":{this.onKeyLine=!0;const s=zu(t),a=er(s);return a.push(this.sourceToken),{type:"block-map",offset:this.offset,indent:this.indent,items:[{start:a,explicitKey:!0}]}}case"map-value-ind":{this.onKeyLine=!0;const s=zu(t),a=er(s);return{type:"block-map",offset:this.offset,indent:this.indent,items:[{start:a,key:null,sep:[this.sourceToken]}]}}}return null}atIndentedComment(t,s){return this.type!=="comment"||this.indent<=s?!1:t.every(a=>a.type==="newline"||a.type==="space")}*documentEnd(t){this.type!=="doc-mode"&&(t.end?t.end.push(this.sourceToken):t.end=[this.sourceToken],this.type==="newline"&&(yield*this.pop()))}*lineEnd(t){switch(this.type){case"comma":case"doc-start":case"doc-end":case"flow-seq-end":case"flow-map-end":case"map-value-ind":yield*this.pop(),yield*this.step();break;case"newline":this.onKeyLine=!1;default:t.end?t.end.push(this.sourceToken):t.end=[this.sourceToken],this.type==="newline"&&(yield*this.pop())}}}function wS(e){const t=e.prettyErrors!==!1;return{lineCounter:e.lineCounter||t&&new vS||null,prettyErrors:t}}function iz(e,t={}){const{lineCounter:s,prettyErrors:a}=wS(t),o=new eg(s?.addNewLine),l=new Wh(t),c=Array.from(l.compose(o.parse(e)));if(a&&s)for(const f of c)f.errors.forEach(pd(e,s)),f.warnings.forEach(pd(e,s));return c.length>0?c:Object.assign([],{empty:!0},l.streamInfo())}function SS(e,t={}){const{lineCounter:s,prettyErrors:a}=wS(t),o=new eg(s?.addNewLine),l=new Wh(t);let c=null;for(const f of l.compose(o.parse(e),!0,e.length))if(!c)c=f;else if(c.options.logLevel!=="silent"){c.errors.push(new Qa(f.range.slice(0,2),"MULTIPLE_DOCS","Source contains multiple documents; please use YAML.parseAllDocuments()"));break}return a&&s&&(c.errors.forEach(pd(e,s)),c.warnings.forEach(pd(e,s))),c}function az(e,t,s){let a;typeof t=="function"?a=t:s===void 0&&t&&typeof t=="object"&&(s=t);const o=SS(e,s);if(!o)return null;if(o.warnings.forEach(l=>Iw(o.options.logLevel,l)),o.errors.length>0){if(o.options.logLevel!=="silent")throw o.errors[0];o.errors=[]}return o.toJS(Object.assign({reviver:a},s))}function oz(e,t,s){let a=null;if(typeof t=="function"||Array.isArray(t)?a=t:s===void 0&&t&&(s=t),typeof s=="string"&&(s=s.length),typeof s=="number"){const o=Math.round(s);s=o<1?void 0:o>8?{indent:8}:{indent:o}}if(e===void 0){const{keepUndefined:o}=s??t??{};if(!o)return}return yo(e)&&!a?e.toString(s):new $r(e,a,s).toString(s)}const kS=Object.freeze(Object.defineProperty({__proto__:null,Alias:Dd,CST:tz,Composer:Wh,Document:$r,Lexer:bS,LineCounter:vS,Pair:jn,Parser:eg,Scalar:Ze,Schema:Zd,YAMLError:Jh,YAMLMap:ns,YAMLParseError:Qa,YAMLSeq:ha,YAMLWarning:lS,isAlias:go,isCollection:$t,isDocument:yo,isMap:Pn,isNode:Rt,isPair:At,isScalar:xt,isSeq:$s,parse:az,parseAllDocuments:iz,parseDocument:SS,stringify:oz,visit:_o,visitAsync:zd},Symbol.toStringTag,{value:"Module"})),rz=["LLM / Agent","多模态生成","内容渲染器","工具与控制流"],Kd=[{type:"llm",label:"LLM 推理 (llm)",category:"LLM / Agent",description:"调用大模型执行提示词生成、结构化 JSON 抽取或改写",defaultInputKey:"context",defaultParameters:{prompt:`请围绕以下上下文生成结构化分析：

{{ context | default("") }}`}},{type:"agent",label:"自主循环 Agent (agent)",category:"LLM / Agent",description:"ReAct / Plan-Execute 多步工具调用智能体节点",defaultInputKey:"task",defaultParameters:{prompt:'{{ task | default("完成调研并输出结论") }}',max_steps:6}},{type:"skill_agent",label:"Skill 专家 (skill_agent)",category:"LLM / Agent",description:"挂载 SKILL.md 领域人设、知识与工具集的智能体",defaultInputKey:"task",defaultParameters:{prompt:'{{ task | default("执行领域专家分析") }}'}},{type:"tts",label:"语音合成 (tts)",category:"多模态生成",description:"将文稿旁白合成为 WAV/MP3 音频文件",defaultInputKey:"text",defaultParameters:{voice:"alloy",input_template:"{{ text }}"}},{type:"text_to_image",label:"文生图 (text_to_image)",category:"多模态生成",description:"根据视觉提示词生成插画或封面图",defaultInputKey:"visual_prompt",defaultParameters:{prompt:"{{ visual_prompt }}",size:"1024x1024"}},{type:"text_to_video",label:"文生视频 (text_to_video)",category:"多模态生成",description:"生成动态视频片段（支持时长与画幅设置）",defaultInputKey:"video_prompt",defaultParameters:{prompt:"{{ video_prompt }}",aspect_ratio:"16:9",duration_seconds:5}},{type:"asr",label:"语音转写 (asr)",category:"多模态生成",description:"将音频输入转写为带时间戳的文本",defaultInputKey:"audio_source",defaultParameters:{audio_source:"{{ audio_source }}"}},{type:"render_slides",label:"幻灯片渲染 (render_slides)",category:"内容渲染器",description:"将结构化 deck JSON 渲染为高清 PNG/SVG 与交互式 HTML 课件",defaultInputKey:"deck",defaultParameters:{width:1920,progressive:!0}},{type:"render_video",label:"课件视频合成 (render_video)",category:"内容渲染器",description:"将幻灯片帧与分段配音通过受控 ffmpeg 合成为旁白视频",defaultInputKey:"slides",defaultParameters:{fps:30}},{type:"render_document",label:"文档/PDF 排版 (render_document)",category:"内容渲染器",description:"通过受控 pandoc / typst 将 Markdown 排版为 PDF 或 HTML 报告",defaultInputKey:"markdown",defaultParameters:{format:"html"}},{type:"render_mindmap",label:"思维导图渲染 (render_mindmap)",category:"内容渲染器",description:"将层级主题树渲染为高清 SVG 思维导图",defaultInputKey:"tree",defaultParameters:{}},{type:"render_study",label:"闪卡/测验渲染 (render_study)",category:"内容渲染器",description:"将题库或闪卡 JSON 渲染为单文件交互式学习网页",defaultInputKey:"study",defaultParameters:{mode:"flashcards"}},{type:"render_audio",label:"播客混音拼接 (render_audio)",category:"内容渲染器",description:"拼接多段对谈音频并执行响度标准化",defaultInputKey:"segments",defaultParameters:{}},{type:"render_svg",label:"SVG 光栅化 (render_svg)",category:"内容渲染器",description:"校验 SVG 矢量图并输出 PNG 预览",defaultInputKey:"svg",defaultParameters:{width:1600}},{type:"render_markdown",label:"Markdown 渲染 (render_markdown)",category:"内容渲染器",description:"将结构化章节拼接输出干净的 Markdown 成品文件",defaultInputKey:"markdown",defaultParameters:{}},{type:"render_table",label:"数据表格渲染 (render_table)",category:"内容渲染器",description:"将行列数据渲染为精美 SVG/HTML 对比矩阵",defaultInputKey:"table",defaultParameters:{}},{type:"figure",label:"代码绘图 (figure)",category:"内容渲染器",description:"LLM 编写 SVG/Typst/TikZ 源码并带编译错误自修复闭环",defaultInputKey:"spec",defaultParameters:{format:"svg"}},{type:"template",label:"Jinja 模板 (template)",category:"工具与控制流",description:"零外部调用的确定性文本/JSON 组装与转换节点",defaultInputKey:"upstream",defaultParameters:{template:'{{ upstream | default("") }}'}},{type:"http",label:"HTTP 请求 (http)",category:"工具与控制流",description:"带 SSRF 防护与 DNS 锁定的外部 API 请求节点",defaultInputKey:"query",defaultParameters:{method:"GET",url:"https://example.com"}},{type:"web_search",label:"联网检索 (web_search)",category:"工具与控制流",description:"多厂商联网搜索（Tavily/Bocha/Serper/Brave + 免 Key 聚合回退）",defaultInputKey:"query",defaultParameters:{query:'{{ query | default("") }}',provider:"auto",max_results:6}},{type:"repo_snapshot",label:"代码仓库快照 (repo_snapshot)",category:"工具与控制流",description:"提取 GitHub 仓库或受控本地源码目录的模块树、Manifest 与依赖边",defaultInputKey:"source",defaultParameters:{source:"https://github.com/tokio-rs/axum"}},{type:"pdf_extract",label:"PDF 论文解析 (pdf_extract)",category:"工具与控制流",description:"沙箱提取 arXiv/OpenReview/本地 PDF 全文与分页结构",defaultInputKey:"source",defaultParameters:{source:"https://arxiv.org/abs/1706.03762"}},{type:"video_ingest",label:"视频/字幕采集 (video_ingest)",category:"工具与控制流",description:"优先提取原生软字幕，自动回退沙箱 ffmpeg 16kHz 音频抽取供 ASR 转写",defaultInputKey:"source",defaultParameters:{source:'{{ source | default("") }}',prefer_subtitles:!0}},{type:"file",label:"受控文件读写 (file)",category:"工具与控制流",description:"在白名单目录内读取或落盘本地文件",defaultInputKey:"content",defaultParameters:{allowed_paths:["./workspace"],operation:"write",path:"./workspace/output.txt"}},{type:"map",label:"Map 并发子流 (map)",category:"工具与控制流",description:"对数组输入逐项（支持并发上限）执行内嵌子流水线",defaultInputKey:"items",defaultParameters:{parallel:!0,max_concurrent:4,template:[{id:"item_step",type:"template",parameters:{template:"{{ item }}"}}]}},{type:"while",label:"While 迭代循环 (while)",category:"工具与控制流",description:"在满足条件且未超最大轮次时循环执行子流水线",defaultInputKey:"seed",defaultParameters:{condition:"false",max_iterations:3,do:[{id:"loop_step",type:"template",parameters:{template:'{{ seed | default("") }}'}}]}}],lz=/^[a-zA-Z0-9_-]+$/;function ac(e){let t;try{t=kS.parseDocument(e)}catch(o){return{ok:!1,error:`YAML 语法解析失败：${o instanceof Error?o.message:String(o)}`}}if(t.errors.length>0)return{ok:!1,error:`YAML 语法错误：${t.errors[0]?.message??"未知错误"}`};if(!t.contents||!Pn(t.contents))return{ok:!1,error:"工作流顶层必须是 YAML 映射对象"};const s=t.contents;let a=s.get("nodes",!0);if(!a){const o=t.createNode([]);s.set("nodes",o),a=o}return $s(a)?{ok:!0,doc:t,root:s,nodesSeq:a}:{ok:!1,error:"工作流 `nodes` 字段必须是数组序列"}}function oc(e){return e.toString({indent:2,lineWidth:0})}function cz(e){const t=e.get("id");return typeof t=="string"&&t.trim()?t.trim():null}function ro(e){const t=[];return e.items.forEach((s,a)=>{if(Pn(s)){const o=cz(s);o&&t.push({id:o,map:s,index:a})}}),t}function Il(e){const t=[],s=/(?:^|[^a-zA-Z0-9_-])nodes\.([a-zA-Z0-9_-]+)/g;let a=s.exec(e);for(;a;){const o=a[1];o&&!t.includes(o)&&t.push(o),a=s.exec(e)}return t}function jS(e){const t=ro(e),s=new Set(t.map(o=>o.id)),a=new Map;for(const o of s)a.set(o,new Set);for(const{id:o,map:l}of t){const c=l.get("dependencies",!0);if($s(c))for(const p of c.items){const _=Ti(p);_&&_!==o&&s.has(_)&&a.get(_)?.add(o)}const f=l.get("input_mapping",!0);if(Pn(f))for(const p of f.items){const _=Ti(p.value);if(_)for(const y of Il(_))y!==o&&s.has(y)&&a.get(y)?.add(o)}const m=l.get("run_if");if(typeof m=="string")for(const p of Il(m))p!==o&&s.has(p)&&a.get(p)?.add(o)}return a}function Ti(e){if(typeof e=="string")return e;if(e&&typeof e=="object"&&"value"in e){const t=e.value;if(typeof t=="string")return t}return null}function uz(e,t,s){if(t===s)return!0;const a=new Set([t]),o=[t];for(;o.length>0;){const l=o.shift();for(const c of e.get(l)??[]){if(c===s)return!0;a.has(c)||(a.add(c),o.push(c))}}return!1}function dz(e){const t=new Map;for(const o of e.keys())t.set(o,0);for(const o of e.values())for(const l of o)t.set(l,(t.get(l)??0)+1);const s=[];for(const[o,l]of t.entries())l===0&&s.push(o);let a=0;for(;s.length>0;){const o=s.shift();a+=1;for(const l of e.get(o)??[]){const c=(t.get(l)??1)-1;t.set(l,c),c===0&&s.push(l)}}return a!==e.size}function NS(e,t,s,a){const l=Kd.find(f=>f.type===s)?.defaultInputKey??"input",c=a.has(l)?`from_${e}`:l;return s==="render_slides"&&c==="deck"?{key:c,expr:`{{ nodes.${e}.outputs.${t==="template"||t==="llm"?"deck":"output"} }}`}:{key:c,expr:`{{ nodes.${e}.outputs.output }}`}}function fz(e,t,s){const a=(s?.trim()||t||"step").replace(/[^a-zA-Z0-9_-]/g,"_");if(s&&!e.has(a))return a;let o=1;for(;e.has(`${a}_${o}`);)o+=1;return`${a}_${o}`}function pz(e,t,s){const a=ac(e);if(!a.ok)return a;const{doc:o,nodesSeq:l}=a,c=ro(l),f=new Set(c.map(b=>b.id)),m=fz(f,t,s?.preferredId),p=Kd.find(b=>b.type===t),_={id:m,type:t},y=s?.afterNodeId?c.find(b=>b.id===s.afterNodeId):void 0;if(y){const b=typeof y.map.get("type")=="string"?y.map.get("type"):"unknown",v=NS(y.id,b,t,new Set);_.dependencies=[y.id],_.input_mapping={[v.key]:v.expr}}return _.parameters=p?structuredClone(p.defaultParameters):{},l.add(o.createNode(_)),{ok:!0,yaml:oc(o),createdNodeId:m}}function mz(e,t,s){if(t===s)return{ok:!1,error:"节点不能连接到自身（禁止自环）"};const a=ac(e);if(!a.ok)return a;const{doc:o,nodesSeq:l}=a,c=ro(l),f=c.find(S=>S.id===t),m=c.find(S=>S.id===s);if(!f||!m)return{ok:!1,error:`未找到待连接节点（${t} → ${s}）`};const p=jS(l);if(uz(p,s,t))return{ok:!1,error:`无法连接 ${t} → ${s}：该连线会形成有向环路（DAG 必须无环）`};const _=m.map.get("dependencies",!0);!_||!$s(_)?m.map.set("dependencies",o.createNode([t])):_.items.map(Ti).filter(w=>!!w).includes(t)||_.add(o.createNode(t));let y=m.map.get("input_mapping",!0);const b=new Set;let v=!1;if(y&&Pn(y))for(const S of y.items){const w=Ti(S.key),k=Ti(S.value);w&&b.add(w),k&&Il(k).includes(t)&&(v=!0)}if(!v){const S=typeof f.map.get("type")=="string"?f.map.get("type"):"unknown",w=typeof m.map.get("type")=="string"?m.map.get("type"):"unknown",k=NS(t,S,w,b);!y||!Pn(y)?m.map.set("input_mapping",o.createNode({[k.key]:k.expr})):y.set(k.key,k.expr)}return{ok:!0,yaml:oc(o)}}function hz(e,t,s){const a=ac(e);if(!a.ok)return a;const{doc:o,nodesSeq:l}=a,c=ro(l).find(f=>f.id===s);return c?(ES(c.map,t),{ok:!0,yaml:oc(o)}):{ok:!1,error:`未找到目标节点 ${s}`}}function ES(e,t){const s=e.get("dependencies",!0);$s(s)&&(s.items=s.items.filter(l=>Ti(l)!==t),s.items.length===0&&e.delete("dependencies"));const a=e.get("input_mapping",!0);Pn(a)&&(a.items=a.items.filter(l=>{const c=Ti(l.value);return!(c&&Il(c).includes(t))}),a.items.length===0&&e.delete("input_mapping"));const o=e.get("run_if");typeof o=="string"&&Il(o).includes(t)&&e.delete("run_if")}function gz(e,t){const s=ac(e);if(!s.ok)return s;const{doc:a,nodesSeq:o}=s,c=ro(o).find(f=>f.id===t);if(!c)return{ok:!1,error:`节点 "${t}" 不存在`};o.items.splice(c.index,1);for(const f of ro(o))ES(f.map,t);return{ok:!0,yaml:oc(a)}}function m0(e,t,s){const a=t.replace(/[.*+?^${}()|[\]\\]/g,"\\$&"),o=new RegExp(`(^|[^a-zA-Z0-9_-])nodes\\.${a}(?=[^a-zA-Z0-9_-]|$)`,"g");return e.replace(o,`$1nodes.${s}`)}function yz(e,t,s){const a=ac(e);if(!a.ok)return a;const{doc:o,nodesSeq:l}=a,c=ro(l),f=c.find(_=>_.id===t);if(!f)return{ok:!1,error:`未找到节点 "${t}"`};let m=t;if(s.newId!==void 0&&s.newId.trim()!==t){const _=s.newId.trim();if(!_||!lz.test(_))return{ok:!1,error:"节点 ID 仅允许字母、数字、下划线与连字符"};if(c.some(y=>y.id===_))return{ok:!1,error:`节点 ID "${_}" 已存在，请换一个名称`};f.map.set("id",_),m=_;for(const y of c){if(y.id===t)continue;const b=y.map.get("dependencies",!0);$s(b)&&(b.items=b.items.map(w=>Ti(w)===t?o.createNode(_):w));const v=y.map.get("input_mapping",!0);if(Pn(v))for(const w of v.items){const k=Ti(w.value);k&&(w.value=o.createNode(m0(k,t,_)))}const S=y.map.get("run_if");typeof S=="string"&&y.map.set("run_if",m0(S,t,_))}}if(s.type!==void 0&&s.type.trim()&&f.map.set("type",s.type.trim()),s.dependencies!==void 0){const _=[...new Set(s.dependencies.map(y=>y.trim()).filter(Boolean))].filter(y=>y!==m);_.length===0?f.map.delete("dependencies"):f.map.set("dependencies",o.createNode(_))}if(s.input_mapping!==void 0){const _=Object.entries(s.input_mapping).map(([y,b])=>[y.trim(),b]).filter(([y])=>!!y);_.length===0?f.map.delete("input_mapping"):f.map.set("input_mapping",o.createNode(Object.fromEntries(_)))}if(s.run_if!==void 0){const _=s.run_if?.trim()??"";_?f.map.set("run_if",_):f.map.delete("run_if")}if(s.timeout_ms!==void 0&&(s.timeout_ms===null||Number.isNaN(s.timeout_ms)||s.timeout_ms<=0?f.map.delete("timeout_ms"):f.map.set("timeout_ms",Math.round(s.timeout_ms))),s.max_retries!==void 0&&(s.max_retries===null||Number.isNaN(s.max_retries)||s.max_retries<0?f.map.delete("max_retries"):f.map.set("max_retries",Math.round(s.max_retries))),s.parameters!==void 0){const _=typeof f.map.get("type")=="string"?f.map.get("type"):"",y=_==="map"?"template":_==="while"?"do":null,b=f.map.get("parameters",!0),v=y&&Pn(b)&&!(y in s.parameters)?b.get(y,!0):void 0,S=o.createNode(s.parameters);y&&v!==void 0&&S.set(y,v),f.map.set("parameters",S)}const p=jS(l);return dz(p)?{ok:!1,error:"该修改会引入节点循环依赖（DAG 必须无环），已拒绝变更"}:{ok:!0,yaml:oc(o),renamedTo:m!==t?m:void 0}}function TS(e,t){return e==="llm"||e==="agent"||e==="skill_agent"||e==="text_to_image"||e==="text_to_video"?"prompt":e==="template"?"template":e==="tts"&&("input_template"in t||Object.keys(t).length===0)?"input_template":null}function h0(e){const t=e.parameters&&typeof e.parameters=="object"&&!Array.isArray(e.parameters)?{...e.parameters}:{},s=TS(e.type,t),a=s&&typeof t[s]=="string"?t[s]:"",o={...t};return s&&delete o[s],{idDraft:e.id,typeDraft:e.type,dependenciesDraft:[...e.dependencies],mappingRows:Object.entries(e.input_mapping).map(([l,c])=>({key:l,expr:c})),runIfDraft:e.run_if??"",timeoutDraft:typeof e.timeout_ms=="number"?String(e.timeout_ms):"",retriesDraft:typeof e.max_retries=="number"?String(e.max_retries):"",quickTextField:s,quickTextValue:a,paramsJsonDraft:JSON.stringify(o,null,2)}}function _z(e,t){const s=t.idDraft.trim();if(!s)return{patch:null,error:"节点 ID 不能为空"};let a={};const o=t.paramsJsonDraft.trim();if(o)try{const p=JSON.parse(o);if(!p||typeof p!="object"||Array.isArray(p))return{patch:null,error:'parameters JSON 必须是对象（如 {"temperature": 0.3}）'};a={...p}}catch(p){return{patch:null,error:`parameters JSON 解析失败：${p instanceof Error?p.message:String(p)}`}}t.quickTextField&&(a[t.quickTextField]=t.quickTextValue);let l=null;if(t.timeoutDraft.trim()){const p=Number(t.timeoutDraft.trim());if(Number.isNaN(p)||p<=0)return{patch:null,error:"超时时间 timeout_ms 必须是正整数（毫秒）"};l=Math.round(p)}let c=null;if(t.retriesDraft.trim()){const p=Number(t.retriesDraft.trim());if(Number.isNaN(p)||p<0)return{patch:null,error:"最大重试次数 max_retries 必须是非负整数"};c=Math.round(p)}const f={};for(const p of t.mappingRows){const _=p.key.trim();_&&(f[_]=p.expr)}return{patch:{newId:s!==e.id?s:void 0,type:t.typeDraft.trim()||e.type,dependencies:t.dependenciesDraft,input_mapping:f,run_if:t.runIfDraft.trim()||null,timeout_ms:l,max_retries:c,parameters:a},error:null}}function bz({node:e,allNodes:t,editable:s,onApplyPatch:a,onDeleteNode:o}){const l=C.useMemo(()=>JSON.stringify(e),[e]),[c,f]=C.useState(()=>h0(e)),[m,p]=C.useState(null),[_,y]=C.useState(!1);C.useEffect(()=>{f(h0(e)),p(null),y(!1)},[l,e]);const b=C.useMemo(()=>t.filter(j=>j.id!==e.id),[t,e.id]),v=C.useMemo(()=>{const j=Kd.map(A=>({value:A.type,label:A.label}));return j.some(A=>A.value===c.typeDraft)||j.unshift({value:c.typeDraft,label:`${c.typeDraft} (自定义)`}),j},[c.typeDraft]),S=()=>{const j=_z(e,c);if(j.error||!j.patch){p(j.error??"配置校验未通过");return}const A=a(e.id,j.patch);if(A){p(A);return}p(null),y(!0)},w=j=>{f(A=>{const T=A.dependenciesDraft.includes(j)?A.dependenciesDraft.filter(E=>E!==j):[...A.dependenciesDraft,j];return{...A,dependenciesDraft:T}}),y(!1)},k=c.quickTextField==="prompt"?"提示词模板 (parameters.prompt)":c.quickTextField==="template"?"Jinja 模板内容 (parameters.template)":"语音输入模板 (parameters.input_template)";return d.jsxs("div",{className:"studio-node-config-editor","data-testid":"node-config-editor",onKeyDown:j=>{(j.metaKey||j.ctrlKey)&&j.key==="Enter"&&s&&(j.preventDefault(),S())},children:[!s&&d.jsx("div",{className:"studio-config-readonly-banner",children:"工作流运行中已锁定拓扑结构编辑，运行结束或终止后可随时修改同步至 YAML。"}),m&&d.jsx("div",{className:"studio-node-error",role:"alert",children:m}),d.jsx("div",{className:"studio-config-section",children:d.jsxs("div",{className:"studio-config-grid-2",children:[d.jsxs("label",{className:"studio-input-field",children:[d.jsxs("span",{className:"studio-input-field__label",children:[d.jsx("strong",{children:"节点 ID"}),d.jsx("span",{children:"级联更新引用"})]}),d.jsx("input",{type:"text",disabled:!s,value:c.idDraft,onChange:j=>{f(A=>({...A,idDraft:j.target.value})),y(!1)}})]}),d.jsxs("label",{className:"studio-input-field",children:[d.jsx("span",{className:"studio-input-field__label",children:d.jsx("strong",{children:"节点类型 (type)"})}),d.jsx("select",{disabled:!s,value:c.typeDraft,onChange:j=>{const A=j.target.value;f(N=>{const T=TS(A,{});return{...N,typeDraft:A,quickTextField:T}}),y(!1)},children:v.map(j=>d.jsx("option",{value:j.value,children:j.label},j.value))})]})]})}),d.jsxs("div",{className:"studio-config-section",children:[d.jsxs("div",{className:"studio-io-block__head",children:[d.jsx("span",{children:"上游依赖节点 (dependencies)"}),d.jsx("small",{children:"也可直接在画布拖拽右端口连线"})]}),b.length===0?d.jsx("p",{className:"studio-muted-note",children:"当前画布仅有 1 个节点，添加更多节点后可配置上下游依赖。"}):d.jsx("div",{className:"studio-dep-pills",children:b.map(j=>{const A=c.dependenciesDraft.includes(j.id);return d.jsxs("button",{type:"button",disabled:!s,"aria-pressed":A,className:`studio-dep-pill ${A?"studio-dep-pill--active":""}`,onClick:()=>w(j.id),children:[A?"✓ ":"+ ",j.id]},j.id)})})]}),d.jsxs("div",{className:"studio-config-section",children:[d.jsxs("div",{className:"studio-io-block__head",children:[d.jsx("span",{children:"输入字段映射 (input_mapping)"}),s&&d.jsx("button",{type:"button",className:"studio-link-btn",onClick:()=>{const j=c.dependenciesDraft[0]??b[0]?.id,A=j?`{{ nodes.${j}.outputs.output }}`:"{{ inputs.topic }}";f(N=>({...N,mappingRows:[...N.mappingRows,{key:`arg_${N.mappingRows.length+1}`,expr:A}]})),y(!1)},children:"+ 新增映射项"})]}),c.mappingRows.length===0?d.jsx("p",{className:"studio-muted-note",children:"暂无显式字段映射（节点直接读取全局 inputs 或静态参数）。"}):d.jsx("div",{className:"studio-mapping-rows",children:c.mappingRows.map((j,A)=>d.jsxs("div",{className:"studio-mapping-row",children:[d.jsx("input",{type:"text","aria-label":`映射参数名 ${A+1}`,placeholder:"字段名",disabled:!s,value:j.key,onChange:N=>{const T=N.target.value;f(E=>({...E,mappingRows:E.mappingRows.map((O,R)=>R===A?{...O,key:T}:O)})),y(!1)}}),d.jsx("span",{className:"studio-mapping-arrow",children:"←"}),d.jsx("input",{type:"text","aria-label":`映射表达式 ${A+1}`,placeholder:"{{ nodes.prev.outputs.output }}",disabled:!s,value:j.expr,onChange:N=>{const T=N.target.value;f(E=>({...E,mappingRows:E.mappingRows.map((O,R)=>R===A?{...O,expr:T}:O)})),y(!1)}}),s&&d.jsx("button",{type:"button",className:"studio-icon-del-btn",title:"移除该映射","aria-label":`移除映射 ${j.key||A+1}`,onClick:()=>{f(N=>({...N,mappingRows:N.mappingRows.filter((T,E)=>E!==A)})),y(!1)},children:"×"})]},A))})]}),c.quickTextField&&d.jsx("div",{className:"studio-config-section",children:d.jsxs("label",{className:"studio-input-field",children:[d.jsx("span",{className:"studio-input-field__label",children:d.jsx("strong",{children:k})}),d.jsx("textarea",{rows:5,disabled:!s,value:c.quickTextValue,onChange:j=>{f(A=>({...A,quickTextValue:j.target.value})),y(!1)}})]})}),d.jsx("div",{className:"studio-config-section",children:d.jsxs("label",{className:"studio-input-field",children:[d.jsx("span",{className:"studio-input-field__label",children:d.jsx("strong",{children:c.quickTextField?"附加静态参数 JSON (parameters)":"静态参数 JSON (parameters)"})}),d.jsx("textarea",{rows:4,className:"studio-mono-textarea",disabled:!s,value:c.paramsJsonDraft,onChange:j=>{f(A=>({...A,paramsJsonDraft:j.target.value})),y(!1)}})]})}),d.jsxs("div",{className:"studio-config-section",children:[d.jsxs("label",{className:"studio-input-field",children:[d.jsxs("span",{className:"studio-input-field__label",children:[d.jsx("strong",{children:"条件守卫 (run_if)"}),d.jsx("span",{children:"optional"})]}),d.jsx("input",{type:"text",placeholder:"例如 {{ nodes.check.outputs.passed }}",disabled:!s,value:c.runIfDraft,onChange:j=>{f(A=>({...A,runIfDraft:j.target.value})),y(!1)}})]}),d.jsxs("div",{className:"studio-config-grid-2",children:[d.jsxs("label",{className:"studio-input-field",children:[d.jsx("span",{className:"studio-input-field__label",children:d.jsx("strong",{children:"超时毫秒 (timeout_ms)"})}),d.jsx("input",{type:"number",min:100,step:1e3,placeholder:"默认无额外限制",disabled:!s,value:c.timeoutDraft,onChange:j=>{f(A=>({...A,timeoutDraft:j.target.value})),y(!1)}})]}),d.jsxs("label",{className:"studio-input-field",children:[d.jsx("span",{className:"studio-input-field__label",children:d.jsx("strong",{children:"最大重试 (max_retries)"})}),d.jsx("input",{type:"number",min:0,max:10,placeholder:"0",disabled:!s,value:c.retriesDraft,onChange:j=>{f(A=>({...A,retriesDraft:j.target.value})),y(!1)}})]})]})]}),e.sub_nodes&&e.sub_nodes.length>0&&d.jsxs("div",{className:"studio-config-section",children:[d.jsx("div",{className:"studio-io-block__head",children:d.jsxs("span",{children:["内嵌子流水线 (",e.sub_nodes.length," 个子节点自动保留)"]})}),d.jsxs("p",{className:"studio-muted-note",children:["子节点：",e.sub_nodes.map(j=>`${j.id} (${j.type})`).join(" → ")]})]}),s&&d.jsxs("div",{className:"studio-config-actions",children:[d.jsx("button",{type:"button","data-testid":"apply-node-config-btn",className:"studio-btn-primary studio-btn-sm",onClick:S,children:"💾 同步到 YAML (⌘/Ctrl+Enter)"}),_&&d.jsx("span",{className:"studio-saved-hint",children:"✓ 已同步至 YAML 与画布"}),o&&d.jsx("button",{type:"button","data-testid":"delete-node-btn",className:"studio-btn-danger-ghost studio-btn-sm",onClick:()=>o(e.id),children:"🗑 删除节点"})]})]})}function Tm(e){if(!e||typeof e!="object"||Array.isArray(e))return{};const t={};for(const[s,a]of Object.entries(e)){if(a&&typeof a=="object"&&!Array.isArray(a)){const o=a;if(o.type==="json"&&"value"in o){t[s]=o.value;continue}}t[s]=a}return t}function vz(e){const t=e.trim();if(!t)return{value:null,error:"覆盖载荷不能为空，请输入合法 JSON 对象"};try{const s=JSON.parse(t);return!s||typeof s!="object"||Array.isArray(s)?{value:null,error:'覆盖载荷顶层必须是 JSON 对象（如 {"output": "..."}）'}:{value:s,error:null}}catch(s){return{value:null,error:`JSON 解析失败：${s instanceof Error?s.message:String(s)}`}}}function xz({runId:e,apiToken:t,tenantId:s,node:a,allNodes:o=[],editable:l=!1,telemetry:c,capturedInput:f,capturedOutput:m,pausedAt:p,hasBreakpoint:_,onToggleBreakpoint:y,onStepDebug:b,onContinueDebug:v,debugActionBusy:S,onApplyNodePatch:w,onDeleteNode:k}){const[j,A]=C.useState(()=>!e&&w?"config":"io"),[N,T]=C.useState(!1),[E,O]=C.useState("input"),[R,q]=C.useState(`{
  "output": ""
}`),[Z,oe]=C.useState(null),Q=C.useMemo(()=>a&&e?Tw(e,a.id,m):[],[e,a,m]);if(C.useEffect(()=>{(e||p)&&A(B=>B==="media"&&Q.length>0?"media":"io")},[e,p?.node_id,p?.phase]),C.useEffect(()=>{if(T(!1),oe(null),!(!p||p.node_id!==a?.id))if(p.phase==="before_node"){O("input");const B=Tm(f);q(JSON.stringify(B,null,2))}else{const B=Tm(m);q(Object.keys(B).length>0?JSON.stringify(B,null,2):`{
  "output": ""
}`)}},[p?.node_id,p?.phase,a?.id,f,m]),C.useEffect(()=>{j==="media"&&Q.length===0&&A(!e&&w?"config":"io")},[a?.id,j,Q.length,e,w]),!a)return d.jsx("section",{className:"studio-inspector studio-inspector--empty","aria-label":"节点配置与观测台",children:d.jsxs("div",{className:"studio-empty-box",children:[d.jsx("h3",{children:"节点配置与 I/O 观测台"}),d.jsx("p",{children:"点击拓扑画布中的任意节点即可表单化编辑参数与依赖连线（实时同步左侧 YAML）；启动「单步调试」后在此实时核查输入、输出与多媒体产物。"})]})});const ee=p?.node_id===a.id,X=f!==void 0,L=m!==void 0,I=B=>{if(!p)return;const V=vz(R);if(V.error||!V.value){oe(V.error??"JSON 格式无效");return}oe(null);const te=p.phase==="before_node"&&E==="input"?{input_override:V.value}:{output_override:V.value};B==="step"?b(te):v(te)},D=[{id:"io",label:"运行 I/O 观测"},{id:"media",label:"媒体预览",badge:Q.length||void 0},{id:"config",label:"节点配置"}],U=B=>{if(B.key!=="ArrowRight"&&B.key!=="ArrowLeft")return;B.preventDefault();const V=D.findIndex(Y=>Y.id===j),te=B.key==="ArrowRight"?1:-1,z=D[(V+te+D.length)%D.length];z&&A(z.id)};return d.jsxs("section",{className:"studio-inspector","aria-label":`节点 ${a.id} 观测详情`,children:[d.jsxs("header",{className:"studio-inspector__header",children:[d.jsxs("div",{className:"studio-inspector__title-group",children:[d.jsxs("div",{className:"studio-inspector__eyebrow",children:[d.jsx("span",{className:"studio-inspector__type-pill",children:a.type}),typeof c?.durationMs=="number"&&d.jsxs("span",{className:"studio-inspector__duration",children:[c.durationMs," ms"]}),typeof c?.costUsd=="number"&&c.costUsd>0&&d.jsxs("span",{className:"studio-inspector__duration",children:["$",c.costUsd.toFixed(c.costUsd<.1?4:3)]})]}),d.jsx("h2",{className:"studio-inspector__title",children:a.id})]}),d.jsxs("button",{type:"button",className:`studio-btn-ghost studio-btn-sm ${_?"studio-btn-bp-on":""}`,"aria-pressed":_,onClick:()=>y(a.id),children:[d.jsx("span",{className:"studio-bp-dot"}),_?"已设断点":"设执行前断点"]})]}),ee&&p&&d.jsxs("div",{className:"studio-pause-callout",role:"status","aria-live":"polite",children:[d.jsxs("div",{className:"studio-pause-callout__text",children:[d.jsx("strong",{children:p.phase==="before_node"?`⏸ 已在执行前断点挂起：${a.id}`:`⏸ 本步执行完毕并暂停：${a.id}`}),d.jsxs("p",{children:[p.phase==="before_node"?"调度器已完成上游表达式求值，可核查下方输入直接放行，或修改输入 / Mock 输出跳过真实外部调用。":"请观测下方实际输入与输出；如需修正结果供下游消费，可展开数据注入覆盖本步输出。",p.next_nodes.length>0&&d.jsxs(d.Fragment,{children:[" 下一就绪节点：",p.next_nodes.join("、"),"。"]})]})]}),d.jsxs("div",{className:"studio-pause-callout__actions",children:[d.jsx("button",{type:"button",className:"studio-btn-primary studio-btn-sm",disabled:S,onClick:()=>b(),children:"▶ 单步下一步"}),d.jsx("button",{type:"button",className:"studio-btn-secondary studio-btn-sm",disabled:S,onClick:()=>v(),children:"⏭ 继续跑完"}),d.jsx("button",{type:"button","data-testid":"toggle-debug-override-btn",className:"studio-btn-ghost studio-btn-sm","aria-expanded":N,onClick:()=>T(B=>!B),children:N?"收起注入":p.phase==="before_node"?"🧪 改输入 / Mock 跳过":"🧪 覆盖输出"})]}),N&&d.jsxs("div",{className:"studio-override-panel","data-testid":"debug-override-panel",children:[p.phase==="before_node"?d.jsxs("div",{className:"studio-override-modes",children:[d.jsx("button",{type:"button",className:`studio-cat-pill ${E==="input"?"active":""}`,onClick:()=>{O("input"),oe(null),q(JSON.stringify(Tm(f),null,2))},children:"✏ 修改输入并执行 (input_override)"}),d.jsx("button",{type:"button",className:`studio-cat-pill ${E==="mock_output"?"active":""}`,onClick:()=>{O("mock_output"),oe(null),q(`{
  "output": "mocked_value"
}`)},children:"⚡ Mock 输出跳过调用 (output_override)"})]}):d.jsxs("small",{className:"studio-override-hint",children:["将替换 #",a.id," 在状态池与 Checkpoint 中的输出，下游节点将读取下方 JSON："]}),d.jsx("textarea",{rows:5,"aria-label":"调试覆盖 JSON 载荷",className:"studio-mono-textarea studio-override-textarea",value:R,onChange:B=>q(B.target.value)}),Z&&d.jsx("p",{className:"studio-override-error",role:"alert",children:Z}),d.jsxs("div",{className:"studio-pause-callout__actions",children:[d.jsx("button",{type:"button","data-testid":"apply-override-step-btn",className:"studio-btn-primary studio-btn-sm",disabled:S,onClick:()=>I("step"),children:p.phase==="before_node"&&E==="mock_output"?"⚡ Mock 跳过并单步":"▶ 注入并单步"}),d.jsx("button",{type:"button","data-testid":"apply-override-continue-btn",className:"studio-btn-secondary studio-btn-sm",disabled:S,onClick:()=>I("continue"),children:"⏭ 注入并继续跑完"})]})]})]}),c?.error&&d.jsxs("div",{className:"studio-node-error",role:"alert",children:[d.jsx("strong",{children:"节点执行报错："})," ",c.error]}),c?.skipReason&&d.jsxs("div",{className:"studio-node-skip-note",children:[d.jsx("strong",{children:"条件跳过原因："})," ",c.skipReason]}),d.jsx("div",{className:"studio-tabs",role:"tablist","aria-label":"节点观测视图切换",onKeyDown:U,children:D.map(B=>{const V=j===B.id;return d.jsxs("button",{type:"button",role:"tab","aria-selected":V,tabIndex:V?0:-1,className:`studio-tab ${V?"studio-tab--active":""}`,onClick:()=>A(B.id),children:[d.jsx("span",{children:B.label}),typeof B.badge=="number"&&d.jsx("em",{className:"studio-tab__badge",children:B.badge})]},B.id)})}),d.jsxs("div",{className:"studio-inspector__body",children:[j==="io"&&d.jsxs("div",{className:"studio-io-stack",children:[d.jsxs("div",{className:"studio-io-block",children:[d.jsxs("div",{className:"studio-io-block__head",children:[d.jsx("span",{children:"实际输入 Input（表达式求值后）"}),d.jsx("small",{children:X?"已捕获":"等待节点启动"})]}),X?d.jsx("pre",{className:"studio-json-box","data-testid":"inspector-node-input",children:As(f)}):d.jsxs("div",{className:"studio-io-placeholder",children:["尚未执行到该节点。映射契约：",Object.keys(a.input_mapping).length>0?d.jsx("pre",{className:"studio-json-box studio-json-box--muted",children:As(a.input_mapping)}):d.jsx("span",{children:"直接读取工作流全局 inputs 与静态 parameters。"})]})]}),d.jsxs("div",{className:"studio-io-block",children:[d.jsxs("div",{className:"studio-io-block__head",children:[d.jsx("span",{children:"实际输出 Output（节点返回载荷）"}),d.jsx("small",{children:L?Q.length>0?`已捕获 · 含 ${Q.length} 项媒体产物`:"已捕获":"等待节点完成"})]}),L?d.jsx("pre",{className:"studio-json-box","data-testid":"inspector-node-output",children:As(m)}):d.jsx("div",{className:"studio-io-placeholder",children:"节点完成后将在此展示完整输出 JSON。"})]}),Q.length>0&&d.jsxs("div",{className:"studio-io-block",children:[d.jsxs("div",{className:"studio-io-block__head",children:[d.jsx("span",{children:"即时产物快览"}),d.jsxs("button",{type:"button",className:"studio-link-btn",onClick:()=>A("media"),children:["查看全部 ",Q.length," 项 →"]})]}),d.jsx("div",{className:"studio-media-list",children:Q.slice(0,2).map(B=>d.jsx(ql,{item:B,apiToken:t,tenantId:s,compact:!0},B.key))})]})]}),j==="media"&&d.jsx("div",{className:"studio-media-list",children:Q.length===0?d.jsx("p",{className:"studio-io-placeholder",children:"当前节点暂无图像、音频、视频或交互式 HTML 产物。"}):Q.map(B=>d.jsx(ql,{item:B,apiToken:t,tenantId:s},B.key))}),j==="config"&&(w?d.jsx(bz,{node:a,allNodes:o,editable:l,onApplyPatch:w,onDeleteNode:k}):d.jsxs("div",{className:"studio-io-stack",children:[a.run_if&&d.jsxs("div",{className:"studio-io-block",children:[d.jsx("div",{className:"studio-io-block__head",children:d.jsx("span",{children:"条件守卫 (run_if)"})}),d.jsx("pre",{className:"studio-json-box",children:a.run_if})]}),d.jsxs("div",{className:"studio-io-block",children:[d.jsxs("div",{className:"studio-io-block__head",children:[d.jsx("span",{children:"输入映射表达式 (input_mapping)"}),d.jsxs("small",{children:[Object.keys(a.input_mapping).length," 项"]})]}),d.jsx("pre",{className:"studio-json-box",children:Object.keys(a.input_mapping).length>0?As(a.input_mapping):"// 无上游显式字段映射"})]}),d.jsxs("div",{className:"studio-io-block",children:[d.jsx("div",{className:"studio-io-block__head",children:d.jsx("span",{children:"静态参数 (parameters)"})}),d.jsx("pre",{className:"studio-json-box",children:As(a.parameters)})]})]}))]})]})}function an(e){if(typeof e=="string"||typeof e=="number")return""+e;let t="";if(Array.isArray(e))for(let s=0,a;s<e.length;s++)(a=an(e[s]))!==""&&(t+=(t&&" ")+a);else for(let s in e)e[s]&&(t+=(t&&" ")+s);return t}var wz={value:()=>{}};function Gd(){for(var e=0,t=arguments.length,s={},a;e<t;++e){if(!(a=arguments[e]+"")||a in s||/[\s.]/.test(a))throw new Error("illegal type: "+a);s[a]=[]}return new Gu(s)}function Gu(e){this._=e}function Sz(e,t){return e.trim().split(/^|\s+/).map(function(s){var a="",o=s.indexOf(".");if(o>=0&&(a=s.slice(o+1),s=s.slice(0,o)),s&&!t.hasOwnProperty(s))throw new Error("unknown type: "+s);return{type:s,name:a}})}Gu.prototype=Gd.prototype={constructor:Gu,on:function(e,t){var s=this._,a=Sz(e+"",s),o,l=-1,c=a.length;if(arguments.length<2){for(;++l<c;)if((o=(e=a[l]).type)&&(o=kz(s[o],e.name)))return o;return}if(t!=null&&typeof t!="function")throw new Error("invalid callback: "+t);for(;++l<c;)if(o=(e=a[l]).type)s[o]=g0(s[o],e.name,t);else if(t==null)for(o in s)s[o]=g0(s[o],e.name,null);return this},copy:function(){var e={},t=this._;for(var s in t)e[s]=t[s].slice();return new Gu(e)},call:function(e,t){if((o=arguments.length-2)>0)for(var s=new Array(o),a=0,o,l;a<o;++a)s[a]=arguments[a+2];if(!this._.hasOwnProperty(e))throw new Error("unknown type: "+e);for(l=this._[e],a=0,o=l.length;a<o;++a)l[a].value.apply(t,s)},apply:function(e,t,s){if(!this._.hasOwnProperty(e))throw new Error("unknown type: "+e);for(var a=this._[e],o=0,l=a.length;o<l;++o)a[o].value.apply(t,s)}};function kz(e,t){for(var s=0,a=e.length,o;s<a;++s)if((o=e[s]).name===t)return o.value}function g0(e,t,s){for(var a=0,o=e.length;a<o;++a)if(e[a].name===t){e[a]=wz,e=e.slice(0,a).concat(e.slice(a+1));break}return s!=null&&e.push({name:t,value:s}),e}var rh="http://www.w3.org/1999/xhtml",y0={svg:"http://www.w3.org/2000/svg",xhtml:rh,xlink:"http://www.w3.org/1999/xlink",xml:"http://www.w3.org/XML/1998/namespace",xmlns:"http://www.w3.org/2000/xmlns/"};function Xd(e){var t=e+="",s=t.indexOf(":");return s>=0&&(t=e.slice(0,s))!=="xmlns"&&(e=e.slice(s+1)),y0.hasOwnProperty(t)?{space:y0[t],local:e}:e}function jz(e){return function(){var t=this.ownerDocument,s=this.namespaceURI;return s===rh&&t.documentElement.namespaceURI===rh?t.createElement(e):t.createElementNS(s,e)}}function Nz(e){return function(){return this.ownerDocument.createElementNS(e.space,e.local)}}function CS(e){var t=Xd(e);return(t.local?Nz:jz)(t)}function Ez(){}function tg(e){return e==null?Ez:function(){return this.querySelector(e)}}function Tz(e){typeof e!="function"&&(e=tg(e));for(var t=this._groups,s=t.length,a=new Array(s),o=0;o<s;++o)for(var l=t[o],c=l.length,f=a[o]=new Array(c),m,p,_=0;_<c;++_)(m=l[_])&&(p=e.call(m,m.__data__,_,l))&&("__data__"in m&&(p.__data__=m.__data__),f[_]=p);return new is(a,this._parents)}function Cz(e){return e==null?[]:Array.isArray(e)?e:Array.from(e)}function Az(){return[]}function AS(e){return e==null?Az:function(){return this.querySelectorAll(e)}}function Oz(e){return function(){return Cz(e.apply(this,arguments))}}function Mz(e){typeof e=="function"?e=Oz(e):e=AS(e);for(var t=this._groups,s=t.length,a=[],o=[],l=0;l<s;++l)for(var c=t[l],f=c.length,m,p=0;p<f;++p)(m=c[p])&&(a.push(e.call(m,m.__data__,p,c)),o.push(m));return new is(a,o)}function OS(e){return function(){return this.matches(e)}}function MS(e){return function(t){return t.matches(e)}}var zz=Array.prototype.find;function Dz(e){return function(){return zz.call(this.children,e)}}function $z(){return this.firstElementChild}function Rz(e){return this.select(e==null?$z:Dz(typeof e=="function"?e:MS(e)))}var Lz=Array.prototype.filter;function qz(){return Array.from(this.children)}function Uz(e){return function(){return Lz.call(this.children,e)}}function Bz(e){return this.selectAll(e==null?qz:Uz(typeof e=="function"?e:MS(e)))}function Hz(e){typeof e!="function"&&(e=OS(e));for(var t=this._groups,s=t.length,a=new Array(s),o=0;o<s;++o)for(var l=t[o],c=l.length,f=a[o]=[],m,p=0;p<c;++p)(m=l[p])&&e.call(m,m.__data__,p,l)&&f.push(m);return new is(a,this._parents)}function zS(e){return new Array(e.length)}function Iz(){return new is(this._enter||this._groups.map(zS),this._parents)}function gd(e,t){this.ownerDocument=e.ownerDocument,this.namespaceURI=e.namespaceURI,this._next=null,this._parent=e,this.__data__=t}gd.prototype={constructor:gd,appendChild:function(e){return this._parent.insertBefore(e,this._next)},insertBefore:function(e,t){return this._parent.insertBefore(e,t)},querySelector:function(e){return this._parent.querySelector(e)},querySelectorAll:function(e){return this._parent.querySelectorAll(e)}};function Zz(e){return function(){return e}}function Pz(e,t,s,a,o,l){for(var c=0,f,m=t.length,p=l.length;c<p;++c)(f=t[c])?(f.__data__=l[c],a[c]=f):s[c]=new gd(e,l[c]);for(;c<m;++c)(f=t[c])&&(o[c]=f)}function Yz(e,t,s,a,o,l,c){var f,m,p=new Map,_=t.length,y=l.length,b=new Array(_),v;for(f=0;f<_;++f)(m=t[f])&&(b[f]=v=c.call(m,m.__data__,f,t)+"",p.has(v)?o[f]=m:p.set(v,m));for(f=0;f<y;++f)v=c.call(e,l[f],f,l)+"",(m=p.get(v))?(a[f]=m,m.__data__=l[f],p.delete(v)):s[f]=new gd(e,l[f]);for(f=0;f<_;++f)(m=t[f])&&p.get(b[f])===m&&(o[f]=m)}function Vz(e){return e.__data__}function Kz(e,t){if(!arguments.length)return Array.from(this,Vz);var s=t?Yz:Pz,a=this._parents,o=this._groups;typeof e!="function"&&(e=Zz(e));for(var l=o.length,c=new Array(l),f=new Array(l),m=new Array(l),p=0;p<l;++p){var _=a[p],y=o[p],b=y.length,v=Gz(e.call(_,_&&_.__data__,p,a)),S=v.length,w=f[p]=new Array(S),k=c[p]=new Array(S),j=m[p]=new Array(b);s(_,y,w,k,j,v,t);for(var A=0,N=0,T,E;A<S;++A)if(T=w[A]){for(A>=N&&(N=A+1);!(E=k[N])&&++N<S;);T._next=E||null}}return c=new is(c,a),c._enter=f,c._exit=m,c}function Gz(e){return typeof e=="object"&&"length"in e?e:Array.from(e)}function Xz(){return new is(this._exit||this._groups.map(zS),this._parents)}function Fz(e,t,s){var a=this.enter(),o=this,l=this.exit();return typeof e=="function"?(a=e(a),a&&(a=a.selection())):a=a.append(e+""),t!=null&&(o=t(o),o&&(o=o.selection())),s==null?l.remove():s(l),a&&o?a.merge(o).order():o}function Jz(e){for(var t=e.selection?e.selection():e,s=this._groups,a=t._groups,o=s.length,l=a.length,c=Math.min(o,l),f=new Array(o),m=0;m<c;++m)for(var p=s[m],_=a[m],y=p.length,b=f[m]=new Array(y),v,S=0;S<y;++S)(v=p[S]||_[S])&&(b[S]=v);for(;m<o;++m)f[m]=s[m];return new is(f,this._parents)}function Qz(){for(var e=this._groups,t=-1,s=e.length;++t<s;)for(var a=e[t],o=a.length-1,l=a[o],c;--o>=0;)(c=a[o])&&(l&&c.compareDocumentPosition(l)^4&&l.parentNode.insertBefore(c,l),l=c);return this}function Wz(e){e||(e=eD);function t(y,b){return y&&b?e(y.__data__,b.__data__):!y-!b}for(var s=this._groups,a=s.length,o=new Array(a),l=0;l<a;++l){for(var c=s[l],f=c.length,m=o[l]=new Array(f),p,_=0;_<f;++_)(p=c[_])&&(m[_]=p);m.sort(t)}return new is(o,this._parents).order()}function eD(e,t){return e<t?-1:e>t?1:e>=t?0:NaN}function tD(){var e=arguments[0];return arguments[0]=this,e.apply(null,arguments),this}function nD(){return Array.from(this)}function sD(){for(var e=this._groups,t=0,s=e.length;t<s;++t)for(var a=e[t],o=0,l=a.length;o<l;++o){var c=a[o];if(c)return c}return null}function iD(){let e=0;for(const t of this)++e;return e}function aD(){return!this.node()}function oD(e){for(var t=this._groups,s=0,a=t.length;s<a;++s)for(var o=t[s],l=0,c=o.length,f;l<c;++l)(f=o[l])&&e.call(f,f.__data__,l,o);return this}function rD(e){return function(){this.removeAttribute(e)}}function lD(e){return function(){this.removeAttributeNS(e.space,e.local)}}function cD(e,t){return function(){this.setAttribute(e,t)}}function uD(e,t){return function(){this.setAttributeNS(e.space,e.local,t)}}function dD(e,t){return function(){var s=t.apply(this,arguments);s==null?this.removeAttribute(e):this.setAttribute(e,s)}}function fD(e,t){return function(){var s=t.apply(this,arguments);s==null?this.removeAttributeNS(e.space,e.local):this.setAttributeNS(e.space,e.local,s)}}function pD(e,t){var s=Xd(e);if(arguments.length<2){var a=this.node();return s.local?a.getAttributeNS(s.space,s.local):a.getAttribute(s)}return this.each((t==null?s.local?lD:rD:typeof t=="function"?s.local?fD:dD:s.local?uD:cD)(s,t))}function DS(e){return e.ownerDocument&&e.ownerDocument.defaultView||e.document&&e||e.defaultView}function mD(e){return function(){this.style.removeProperty(e)}}function hD(e,t,s){return function(){this.style.setProperty(e,t,s)}}function gD(e,t,s){return function(){var a=t.apply(this,arguments);a==null?this.style.removeProperty(e):this.style.setProperty(e,a,s)}}function yD(e,t,s){return arguments.length>1?this.each((t==null?mD:typeof t=="function"?gD:hD)(e,t,s??"")):xr(this.node(),e)}function xr(e,t){return e.style.getPropertyValue(t)||DS(e).getComputedStyle(e,null).getPropertyValue(t)}function _D(e){return function(){delete this[e]}}function bD(e,t){return function(){this[e]=t}}function vD(e,t){return function(){var s=t.apply(this,arguments);s==null?delete this[e]:this[e]=s}}function xD(e,t){return arguments.length>1?this.each((t==null?_D:typeof t=="function"?vD:bD)(e,t)):this.node()[e]}function $S(e){return e.trim().split(/^|\s+/)}function ng(e){return e.classList||new RS(e)}function RS(e){this._node=e,this._names=$S(e.getAttribute("class")||"")}RS.prototype={add:function(e){var t=this._names.indexOf(e);t<0&&(this._names.push(e),this._node.setAttribute("class",this._names.join(" ")))},remove:function(e){var t=this._names.indexOf(e);t>=0&&(this._names.splice(t,1),this._node.setAttribute("class",this._names.join(" ")))},contains:function(e){return this._names.indexOf(e)>=0}};function LS(e,t){for(var s=ng(e),a=-1,o=t.length;++a<o;)s.add(t[a])}function qS(e,t){for(var s=ng(e),a=-1,o=t.length;++a<o;)s.remove(t[a])}function wD(e){return function(){LS(this,e)}}function SD(e){return function(){qS(this,e)}}function kD(e,t){return function(){(t.apply(this,arguments)?LS:qS)(this,e)}}function jD(e,t){var s=$S(e+"");if(arguments.length<2){for(var a=ng(this.node()),o=-1,l=s.length;++o<l;)if(!a.contains(s[o]))return!1;return!0}return this.each((typeof t=="function"?kD:t?wD:SD)(s,t))}function ND(){this.textContent=""}function ED(e){return function(){this.textContent=e}}function TD(e){return function(){var t=e.apply(this,arguments);this.textContent=t??""}}function CD(e){return arguments.length?this.each(e==null?ND:(typeof e=="function"?TD:ED)(e)):this.node().textContent}function AD(){this.innerHTML=""}function OD(e){return function(){this.innerHTML=e}}function MD(e){return function(){var t=e.apply(this,arguments);this.innerHTML=t??""}}function zD(e){return arguments.length?this.each(e==null?AD:(typeof e=="function"?MD:OD)(e)):this.node().innerHTML}function DD(){this.nextSibling&&this.parentNode.appendChild(this)}function $D(){return this.each(DD)}function RD(){this.previousSibling&&this.parentNode.insertBefore(this,this.parentNode.firstChild)}function LD(){return this.each(RD)}function qD(e){var t=typeof e=="function"?e:CS(e);return this.select(function(){return this.appendChild(t.apply(this,arguments))})}function UD(){return null}function BD(e,t){var s=typeof e=="function"?e:CS(e),a=t==null?UD:typeof t=="function"?t:tg(t);return this.select(function(){return this.insertBefore(s.apply(this,arguments),a.apply(this,arguments)||null)})}function HD(){var e=this.parentNode;e&&e.removeChild(this)}function ID(){return this.each(HD)}function ZD(){var e=this.cloneNode(!1),t=this.parentNode;return t?t.insertBefore(e,this.nextSibling):e}function PD(){var e=this.cloneNode(!0),t=this.parentNode;return t?t.insertBefore(e,this.nextSibling):e}function YD(e){return this.select(e?PD:ZD)}function VD(e){return arguments.length?this.property("__data__",e):this.node().__data__}function KD(e){return function(t){e.call(this,t,this.__data__)}}function GD(e){return e.trim().split(/^|\s+/).map(function(t){var s="",a=t.indexOf(".");return a>=0&&(s=t.slice(a+1),t=t.slice(0,a)),{type:t,name:s}})}function XD(e){return function(){var t=this.__on;if(t){for(var s=0,a=-1,o=t.length,l;s<o;++s)l=t[s],(!e.type||l.type===e.type)&&l.name===e.name?this.removeEventListener(l.type,l.listener,l.options):t[++a]=l;++a?t.length=a:delete this.__on}}}function FD(e,t,s){return function(){var a=this.__on,o,l=KD(t);if(a){for(var c=0,f=a.length;c<f;++c)if((o=a[c]).type===e.type&&o.name===e.name){this.removeEventListener(o.type,o.listener,o.options),this.addEventListener(o.type,o.listener=l,o.options=s),o.value=t;return}}this.addEventListener(e.type,l,s),o={type:e.type,name:e.name,value:t,listener:l,options:s},a?a.push(o):this.__on=[o]}}function JD(e,t,s){var a=GD(e+""),o,l=a.length,c;if(arguments.length<2){var f=this.node().__on;if(f){for(var m=0,p=f.length,_;m<p;++m)for(o=0,_=f[m];o<l;++o)if((c=a[o]).type===_.type&&c.name===_.name)return _.value}return}for(f=t?FD:XD,o=0;o<l;++o)this.each(f(a[o],t,s));return this}function US(e,t,s){var a=DS(e),o=a.CustomEvent;typeof o=="function"?o=new o(t,s):(o=a.document.createEvent("Event"),s?(o.initEvent(t,s.bubbles,s.cancelable),o.detail=s.detail):o.initEvent(t,!1,!1)),e.dispatchEvent(o)}function QD(e,t){return function(){return US(this,e,t)}}function WD(e,t){return function(){return US(this,e,t.apply(this,arguments))}}function e$(e,t){return this.each((typeof t=="function"?WD:QD)(e,t))}function*t$(){for(var e=this._groups,t=0,s=e.length;t<s;++t)for(var a=e[t],o=0,l=a.length,c;o<l;++o)(c=a[o])&&(yield c)}var BS=[null];function is(e,t){this._groups=e,this._parents=t}function rc(){return new is([[document.documentElement]],BS)}function n$(){return this}is.prototype=rc.prototype={constructor:is,select:Tz,selectAll:Mz,selectChild:Rz,selectChildren:Bz,filter:Hz,data:Kz,enter:Iz,exit:Xz,join:Fz,merge:Jz,selection:n$,order:Qz,sort:Wz,call:tD,nodes:nD,node:sD,size:iD,empty:aD,each:oD,attr:pD,style:yD,property:xD,classed:jD,text:CD,html:zD,raise:$D,lower:LD,append:qD,insert:BD,remove:ID,clone:YD,datum:VD,on:JD,dispatch:e$,[Symbol.iterator]:t$};function ss(e){return typeof e=="string"?new is([[document.querySelector(e)]],[document.documentElement]):new is([[e]],BS)}var s$=0;function _0(){this._="@"+(++s$).toString(36)}_0.prototype={constructor:_0,get:function(e){for(var t=this._;!(t in e);)if(!(e=e.parentNode))return;return e[t]},set:function(e,t){return e[this._]=t},remove:function(e){return this._ in e&&delete e[this._]},toString:function(){return this._}};function i$(e){let t;for(;t=e.sourceEvent;)e=t;return e}function Cs(e,t){if(e=i$(e),t===void 0&&(t=e.currentTarget),t){var s=t.ownerSVGElement||t;if(s.createSVGPoint){var a=s.createSVGPoint();return a.x=e.clientX,a.y=e.clientY,a=a.matrixTransform(t.getScreenCTM().inverse()),[a.x,a.y]}if(t.getBoundingClientRect){var o=t.getBoundingClientRect();return[e.clientX-o.left-t.clientLeft,e.clientY-o.top-t.clientTop]}}return[e.pageX,e.pageY]}const a$={passive:!1},Zl={capture:!0,passive:!1};function Cm(e){e.stopImmediatePropagation()}function mr(e){e.preventDefault(),e.stopImmediatePropagation()}function HS(e){var t=e.document.documentElement,s=ss(e).on("dragstart.drag",mr,Zl);"onselectstart"in t?s.on("selectstart.drag",mr,Zl):(t.__noselect=t.style.MozUserSelect,t.style.MozUserSelect="none")}function IS(e,t){var s=e.document.documentElement,a=ss(e).on("dragstart.drag",null);t&&(a.on("click.drag",mr,Zl),setTimeout(function(){a.on("click.drag",null)},0)),"onselectstart"in s?a.on("selectstart.drag",null):(s.style.MozUserSelect=s.__noselect,delete s.__noselect)}var Du=e=>()=>e;function lh(e,{sourceEvent:t,subject:s,target:a,identifier:o,active:l,x:c,y:f,dx:m,dy:p,dispatch:_}){Object.defineProperties(this,{type:{value:e,enumerable:!0,configurable:!0},sourceEvent:{value:t,enumerable:!0,configurable:!0},subject:{value:s,enumerable:!0,configurable:!0},target:{value:a,enumerable:!0,configurable:!0},identifier:{value:o,enumerable:!0,configurable:!0},active:{value:l,enumerable:!0,configurable:!0},x:{value:c,enumerable:!0,configurable:!0},y:{value:f,enumerable:!0,configurable:!0},dx:{value:m,enumerable:!0,configurable:!0},dy:{value:p,enumerable:!0,configurable:!0},_:{value:_}})}lh.prototype.on=function(){var e=this._.on.apply(this._,arguments);return e===this._?this:e};function o$(e){return!e.ctrlKey&&!e.button}function r$(){return this.parentNode}function l$(e,t){return t??{x:e.x,y:e.y}}function c$(){return navigator.maxTouchPoints||"ontouchstart"in this}function ZS(){var e=o$,t=r$,s=l$,a=c$,o={},l=Gd("start","drag","end"),c=0,f,m,p,_,y=0;function b(T){T.on("mousedown.drag",v).filter(a).on("touchstart.drag",k).on("touchmove.drag",j,a$).on("touchend.drag touchcancel.drag",A).style("touch-action","none").style("-webkit-tap-highlight-color","rgba(0,0,0,0)")}function v(T,E){if(!(_||!e.call(this,T,E))){var O=N(this,t.call(this,T,E),T,E,"mouse");O&&(ss(T.view).on("mousemove.drag",S,Zl).on("mouseup.drag",w,Zl),HS(T.view),Cm(T),p=!1,f=T.clientX,m=T.clientY,O("start",T))}}function S(T){if(mr(T),!p){var E=T.clientX-f,O=T.clientY-m;p=E*E+O*O>y}o.mouse("drag",T)}function w(T){ss(T.view).on("mousemove.drag mouseup.drag",null),IS(T.view,p),mr(T),o.mouse("end",T)}function k(T,E){if(e.call(this,T,E)){var O=T.changedTouches,R=t.call(this,T,E),q=O.length,Z,oe;for(Z=0;Z<q;++Z)(oe=N(this,R,T,E,O[Z].identifier,O[Z]))&&(Cm(T),oe("start",T,O[Z]))}}function j(T){var E=T.changedTouches,O=E.length,R,q;for(R=0;R<O;++R)(q=o[E[R].identifier])&&(mr(T),q("drag",T,E[R]))}function A(T){var E=T.changedTouches,O=E.length,R,q;for(_&&clearTimeout(_),_=setTimeout(function(){_=null},500),R=0;R<O;++R)(q=o[E[R].identifier])&&(Cm(T),q("end",T,E[R]))}function N(T,E,O,R,q,Z){var oe=l.copy(),Q=Cs(Z||O,E),ee,X,L;if((L=s.call(T,new lh("beforestart",{sourceEvent:O,target:b,identifier:q,active:c,x:Q[0],y:Q[1],dx:0,dy:0,dispatch:oe}),R))!=null)return ee=L.x-Q[0]||0,X=L.y-Q[1]||0,function I(D,U,B){var V=Q,te;switch(D){case"start":o[q]=I,te=c++;break;case"end":delete o[q],--c;case"drag":Q=Cs(B||U,E),te=c;break}oe.call(D,T,new lh(D,{sourceEvent:U,subject:L,target:b,identifier:q,active:te,x:Q[0]+ee,y:Q[1]+X,dx:Q[0]-V[0],dy:Q[1]-V[1],dispatch:oe}),R)}}return b.filter=function(T){return arguments.length?(e=typeof T=="function"?T:Du(!!T),b):e},b.container=function(T){return arguments.length?(t=typeof T=="function"?T:Du(T),b):t},b.subject=function(T){return arguments.length?(s=typeof T=="function"?T:Du(T),b):s},b.touchable=function(T){return arguments.length?(a=typeof T=="function"?T:Du(!!T),b):a},b.on=function(){var T=l.on.apply(l,arguments);return T===l?b:T},b.clickDistance=function(T){return arguments.length?(y=(T=+T)*T,b):Math.sqrt(y)},b}function Rr(e,t,s){e.prototype=t.prototype=s,s.constructor=e}function lc(e,t){var s=Object.create(e.prototype);for(var a in t)s[a]=t[a];return s}function ba(){}var lo=.7,wr=1/lo,hr="\\s*([+-]?\\d+)\\s*",Pl="\\s*([+-]?(?:\\d*\\.)?\\d+(?:[eE][+-]?\\d+)?)\\s*",Xs="\\s*([+-]?(?:\\d*\\.)?\\d+(?:[eE][+-]?\\d+)?)%\\s*",u$=/^#([0-9a-f]{3,8})$/,d$=new RegExp(`^rgb\\(${hr},${hr},${hr}\\)$`),f$=new RegExp(`^rgb\\(${Xs},${Xs},${Xs}\\)$`),p$=new RegExp(`^rgba\\(${hr},${hr},${hr},${Pl}\\)$`),m$=new RegExp(`^rgba\\(${Xs},${Xs},${Xs},${Pl}\\)$`),h$=new RegExp(`^hsl\\(${Pl},${Xs},${Xs}\\)$`),g$=new RegExp(`^hsla\\(${Pl},${Xs},${Xs},${Pl}\\)$`),b0={aliceblue:15792383,antiquewhite:16444375,aqua:65535,aquamarine:8388564,azure:15794175,beige:16119260,bisque:16770244,black:0,blanchedalmond:16772045,blue:255,blueviolet:9055202,brown:10824234,burlywood:14596231,cadetblue:6266528,chartreuse:8388352,chocolate:13789470,coral:16744272,cornflowerblue:6591981,cornsilk:16775388,crimson:14423100,cyan:65535,darkblue:139,darkcyan:35723,darkgoldenrod:12092939,darkgray:11119017,darkgreen:25600,darkgrey:11119017,darkkhaki:12433259,darkmagenta:9109643,darkolivegreen:5597999,darkorange:16747520,darkorchid:10040012,darkred:9109504,darksalmon:15308410,darkseagreen:9419919,darkslateblue:4734347,darkslategray:3100495,darkslategrey:3100495,darkturquoise:52945,darkviolet:9699539,deeppink:16716947,deepskyblue:49151,dimgray:6908265,dimgrey:6908265,dodgerblue:2003199,firebrick:11674146,floralwhite:16775920,forestgreen:2263842,fuchsia:16711935,gainsboro:14474460,ghostwhite:16316671,gold:16766720,goldenrod:14329120,gray:8421504,green:32768,greenyellow:11403055,grey:8421504,honeydew:15794160,hotpink:16738740,indianred:13458524,indigo:4915330,ivory:16777200,khaki:15787660,lavender:15132410,lavenderblush:16773365,lawngreen:8190976,lemonchiffon:16775885,lightblue:11393254,lightcoral:15761536,lightcyan:14745599,lightgoldenrodyellow:16448210,lightgray:13882323,lightgreen:9498256,lightgrey:13882323,lightpink:16758465,lightsalmon:16752762,lightseagreen:2142890,lightskyblue:8900346,lightslategray:7833753,lightslategrey:7833753,lightsteelblue:11584734,lightyellow:16777184,lime:65280,limegreen:3329330,linen:16445670,magenta:16711935,maroon:8388608,mediumaquamarine:6737322,mediumblue:205,mediumorchid:12211667,mediumpurple:9662683,mediumseagreen:3978097,mediumslateblue:8087790,mediumspringgreen:64154,mediumturquoise:4772300,mediumvioletred:13047173,midnightblue:1644912,mintcream:16121850,mistyrose:16770273,moccasin:16770229,navajowhite:16768685,navy:128,oldlace:16643558,olive:8421376,olivedrab:7048739,orange:16753920,orangered:16729344,orchid:14315734,palegoldenrod:15657130,palegreen:10025880,paleturquoise:11529966,palevioletred:14381203,papayawhip:16773077,peachpuff:16767673,peru:13468991,pink:16761035,plum:14524637,powderblue:11591910,purple:8388736,rebeccapurple:6697881,red:16711680,rosybrown:12357519,royalblue:4286945,saddlebrown:9127187,salmon:16416882,sandybrown:16032864,seagreen:3050327,seashell:16774638,sienna:10506797,silver:12632256,skyblue:8900331,slateblue:6970061,slategray:7372944,slategrey:7372944,snow:16775930,springgreen:65407,steelblue:4620980,tan:13808780,teal:32896,thistle:14204888,tomato:16737095,turquoise:4251856,violet:15631086,wheat:16113331,white:16777215,whitesmoke:16119285,yellow:16776960,yellowgreen:10145074};Rr(ba,co,{copy(e){return Object.assign(new this.constructor,this,e)},displayable(){return this.rgb().displayable()},hex:v0,formatHex:v0,formatHex8:y$,formatHsl:_$,formatRgb:x0,toString:x0});function v0(){return this.rgb().formatHex()}function y$(){return this.rgb().formatHex8()}function _$(){return PS(this).formatHsl()}function x0(){return this.rgb().formatRgb()}function co(e){var t,s;return e=(e+"").trim().toLowerCase(),(t=u$.exec(e))?(s=t[1].length,t=parseInt(t[1],16),s===6?w0(t):s===3?new pn(t>>8&15|t>>4&240,t>>4&15|t&240,(t&15)<<4|t&15,1):s===8?$u(t>>24&255,t>>16&255,t>>8&255,(t&255)/255):s===4?$u(t>>12&15|t>>8&240,t>>8&15|t>>4&240,t>>4&15|t&240,((t&15)<<4|t&15)/255):null):(t=d$.exec(e))?new pn(t[1],t[2],t[3],1):(t=f$.exec(e))?new pn(t[1]*255/100,t[2]*255/100,t[3]*255/100,1):(t=p$.exec(e))?$u(t[1],t[2],t[3],t[4]):(t=m$.exec(e))?$u(t[1]*255/100,t[2]*255/100,t[3]*255/100,t[4]):(t=h$.exec(e))?j0(t[1],t[2]/100,t[3]/100,1):(t=g$.exec(e))?j0(t[1],t[2]/100,t[3]/100,t[4]):b0.hasOwnProperty(e)?w0(b0[e]):e==="transparent"?new pn(NaN,NaN,NaN,0):null}function w0(e){return new pn(e>>16&255,e>>8&255,e&255,1)}function $u(e,t,s,a){return a<=0&&(e=t=s=NaN),new pn(e,t,s,a)}function sg(e){return e instanceof ba||(e=co(e)),e?(e=e.rgb(),new pn(e.r,e.g,e.b,e.opacity)):new pn}function ch(e,t,s,a){return arguments.length===1?sg(e):new pn(e,t,s,a??1)}function pn(e,t,s,a){this.r=+e,this.g=+t,this.b=+s,this.opacity=+a}Rr(pn,ch,lc(ba,{brighter(e){return e=e==null?wr:Math.pow(wr,e),new pn(this.r*e,this.g*e,this.b*e,this.opacity)},darker(e){return e=e==null?lo:Math.pow(lo,e),new pn(this.r*e,this.g*e,this.b*e,this.opacity)},rgb(){return this},clamp(){return new pn(to(this.r),to(this.g),to(this.b),yd(this.opacity))},displayable(){return-.5<=this.r&&this.r<255.5&&-.5<=this.g&&this.g<255.5&&-.5<=this.b&&this.b<255.5&&0<=this.opacity&&this.opacity<=1},hex:S0,formatHex:S0,formatHex8:b$,formatRgb:k0,toString:k0}));function S0(){return`#${Wa(this.r)}${Wa(this.g)}${Wa(this.b)}`}function b$(){return`#${Wa(this.r)}${Wa(this.g)}${Wa(this.b)}${Wa((isNaN(this.opacity)?1:this.opacity)*255)}`}function k0(){const e=yd(this.opacity);return`${e===1?"rgb(":"rgba("}${to(this.r)}, ${to(this.g)}, ${to(this.b)}${e===1?")":`, ${e})`}`}function yd(e){return isNaN(e)?1:Math.max(0,Math.min(1,e))}function to(e){return Math.max(0,Math.min(255,Math.round(e)||0))}function Wa(e){return e=to(e),(e<16?"0":"")+e.toString(16)}function j0(e,t,s,a){return a<=0?e=t=s=NaN:s<=0||s>=1?e=t=NaN:t<=0&&(e=NaN),new Os(e,t,s,a)}function PS(e){if(e instanceof Os)return new Os(e.h,e.s,e.l,e.opacity);if(e instanceof ba||(e=co(e)),!e)return new Os;if(e instanceof Os)return e;e=e.rgb();var t=e.r/255,s=e.g/255,a=e.b/255,o=Math.min(t,s,a),l=Math.max(t,s,a),c=NaN,f=l-o,m=(l+o)/2;return f?(t===l?c=(s-a)/f+(s<a)*6:s===l?c=(a-t)/f+2:c=(t-s)/f+4,f/=m<.5?l+o:2-l-o,c*=60):f=m>0&&m<1?0:c,new Os(c,f,m,e.opacity)}function v$(e,t,s,a){return arguments.length===1?PS(e):new Os(e,t,s,a??1)}function Os(e,t,s,a){this.h=+e,this.s=+t,this.l=+s,this.opacity=+a}Rr(Os,v$,lc(ba,{brighter(e){return e=e==null?wr:Math.pow(wr,e),new Os(this.h,this.s,this.l*e,this.opacity)},darker(e){return e=e==null?lo:Math.pow(lo,e),new Os(this.h,this.s,this.l*e,this.opacity)},rgb(){var e=this.h%360+(this.h<0)*360,t=isNaN(e)||isNaN(this.s)?0:this.s,s=this.l,a=s+(s<.5?s:1-s)*t,o=2*s-a;return new pn(Am(e>=240?e-240:e+120,o,a),Am(e,o,a),Am(e<120?e+240:e-120,o,a),this.opacity)},clamp(){return new Os(N0(this.h),Ru(this.s),Ru(this.l),yd(this.opacity))},displayable(){return(0<=this.s&&this.s<=1||isNaN(this.s))&&0<=this.l&&this.l<=1&&0<=this.opacity&&this.opacity<=1},formatHsl(){const e=yd(this.opacity);return`${e===1?"hsl(":"hsla("}${N0(this.h)}, ${Ru(this.s)*100}%, ${Ru(this.l)*100}%${e===1?")":`, ${e})`}`}}));function N0(e){return e=(e||0)%360,e<0?e+360:e}function Ru(e){return Math.max(0,Math.min(1,e||0))}function Am(e,t,s){return(e<60?t+(s-t)*e/60:e<180?s:e<240?t+(s-t)*(240-e)/60:t)*255}const YS=Math.PI/180,VS=180/Math.PI,_d=18,KS=.96422,GS=1,XS=.82521,FS=4/29,gr=6/29,JS=3*gr*gr,x$=gr*gr*gr;function QS(e){if(e instanceof Fs)return new Fs(e.l,e.a,e.b,e.opacity);if(e instanceof ki)return WS(e);e instanceof pn||(e=sg(e));var t=Dm(e.r),s=Dm(e.g),a=Dm(e.b),o=Om((.2225045*t+.7168786*s+.0606169*a)/GS),l,c;return t===s&&s===a?l=c=o:(l=Om((.4360747*t+.3850649*s+.1430804*a)/KS),c=Om((.0139322*t+.0971045*s+.7141733*a)/XS)),new Fs(116*o-16,500*(l-o),200*(o-c),e.opacity)}function w$(e,t,s,a){return arguments.length===1?QS(e):new Fs(e,t,s,a??1)}function Fs(e,t,s,a){this.l=+e,this.a=+t,this.b=+s,this.opacity=+a}Rr(Fs,w$,lc(ba,{brighter(e){return new Fs(this.l+_d*(e??1),this.a,this.b,this.opacity)},darker(e){return new Fs(this.l-_d*(e??1),this.a,this.b,this.opacity)},rgb(){var e=(this.l+16)/116,t=isNaN(this.a)?e:e+this.a/500,s=isNaN(this.b)?e:e-this.b/200;return t=KS*Mm(t),e=GS*Mm(e),s=XS*Mm(s),new pn(zm(3.1338561*t-1.6168667*e-.4906146*s),zm(-.9787684*t+1.9161415*e+.033454*s),zm(.0719453*t-.2289914*e+1.4052427*s),this.opacity)}}));function Om(e){return e>x$?Math.pow(e,1/3):e/JS+FS}function Mm(e){return e>gr?e*e*e:JS*(e-FS)}function zm(e){return 255*(e<=.0031308?12.92*e:1.055*Math.pow(e,1/2.4)-.055)}function Dm(e){return(e/=255)<=.04045?e/12.92:Math.pow((e+.055)/1.055,2.4)}function S$(e){if(e instanceof ki)return new ki(e.h,e.c,e.l,e.opacity);if(e instanceof Fs||(e=QS(e)),e.a===0&&e.b===0)return new ki(NaN,0<e.l&&e.l<100?0:NaN,e.l,e.opacity);var t=Math.atan2(e.b,e.a)*VS;return new ki(t<0?t+360:t,Math.sqrt(e.a*e.a+e.b*e.b),e.l,e.opacity)}function k$(e,t,s,a){return arguments.length===1?S$(e):new ki(e,t,s,a??1)}function ki(e,t,s,a){this.h=+e,this.c=+t,this.l=+s,this.opacity=+a}function WS(e){if(isNaN(e.h))return new Fs(e.l,0,0,e.opacity);var t=e.h*YS;return new Fs(e.l,Math.cos(t)*e.c,Math.sin(t)*e.c,e.opacity)}Rr(ki,k$,lc(ba,{brighter(e){return new ki(this.h,this.c,this.l+_d*(e??1),this.opacity)},darker(e){return new ki(this.h,this.c,this.l-_d*(e??1),this.opacity)},rgb(){return WS(this).rgb()}}));var e1=-.14861,ig=1.78277,ag=-.29227,Fd=-.90649,Yl=1.97294,E0=Yl*Fd,T0=Yl*ig,C0=ig*ag-Fd*e1;function j$(e){if(e instanceof no)return new no(e.h,e.s,e.l,e.opacity);e instanceof pn||(e=sg(e));var t=e.r/255,s=e.g/255,a=e.b/255,o=(C0*a+E0*t-T0*s)/(C0+E0-T0),l=a-o,c=(Yl*(s-o)-ag*l)/Fd,f=Math.sqrt(c*c+l*l)/(Yl*o*(1-o)),m=f?Math.atan2(c,l)*VS-120:NaN;return new no(m<0?m+360:m,f,o,e.opacity)}function uh(e,t,s,a){return arguments.length===1?j$(e):new no(e,t,s,a??1)}function no(e,t,s,a){this.h=+e,this.s=+t,this.l=+s,this.opacity=+a}Rr(no,uh,lc(ba,{brighter(e){return e=e==null?wr:Math.pow(wr,e),new no(this.h,this.s,this.l*e,this.opacity)},darker(e){return e=e==null?lo:Math.pow(lo,e),new no(this.h,this.s,this.l*e,this.opacity)},rgb(){var e=isNaN(this.h)?0:(this.h+120)*YS,t=+this.l,s=isNaN(this.s)?0:this.s*t*(1-t),a=Math.cos(e),o=Math.sin(e);return new pn(255*(t+s*(e1*a+ig*o)),255*(t+s*(ag*a+Fd*o)),255*(t+s*(Yl*a)),this.opacity)}}));var Jd=e=>()=>e;function t1(e,t){return function(s){return e+s*t}}function N$(e,t,s){return e=Math.pow(e,s),t=Math.pow(t,s)-e,s=1/s,function(a){return Math.pow(e+a*t,s)}}function E$(e,t){var s=t-e;return s?t1(e,s>180||s<-180?s-360*Math.round(s/360):s):Jd(isNaN(e)?t:e)}function T$(e){return(e=+e)==1?yr:function(t,s){return s-t?N$(t,s,e):Jd(isNaN(t)?s:t)}}function yr(e,t){var s=t-e;return s?t1(e,s):Jd(isNaN(e)?t:e)}var bd=(function e(t){var s=T$(t);function a(o,l){var c=s((o=ch(o)).r,(l=ch(l)).r),f=s(o.g,l.g),m=s(o.b,l.b),p=yr(o.opacity,l.opacity);return function(_){return o.r=c(_),o.g=f(_),o.b=m(_),o.opacity=p(_),o+""}}return a.gamma=e,a})(1);function C$(e,t){t||(t=[]);var s=e?Math.min(t.length,e.length):0,a=t.slice(),o;return function(l){for(o=0;o<s;++o)a[o]=e[o]*(1-l)+t[o]*l;return a}}function A$(e){return ArrayBuffer.isView(e)&&!(e instanceof DataView)}function O$(e,t){var s=t?t.length:0,a=e?Math.min(s,e.length):0,o=new Array(a),l=new Array(s),c;for(c=0;c<a;++c)o[c]=$l(e[c],t[c]);for(;c<s;++c)l[c]=t[c];return function(f){for(c=0;c<a;++c)l[c]=o[c](f);return l}}function M$(e,t){var s=new Date;return e=+e,t=+t,function(a){return s.setTime(e*(1-a)+t*a),s}}function Ks(e,t){return e=+e,t=+t,function(s){return e*(1-s)+t*s}}function z$(e,t){var s={},a={},o;(e===null||typeof e!="object")&&(e={}),(t===null||typeof t!="object")&&(t={});for(o in t)o in e?s[o]=$l(e[o],t[o]):a[o]=t[o];return function(l){for(o in s)a[o]=s[o](l);return a}}var dh=/[-+]?(?:\d+\.?\d*|\.?\d+)(?:[eE][-+]?\d+)?/g,$m=new RegExp(dh.source,"g");function D$(e){return function(){return e}}function $$(e){return function(t){return e(t)+""}}function n1(e,t){var s=dh.lastIndex=$m.lastIndex=0,a,o,l,c=-1,f=[],m=[];for(e=e+"",t=t+"";(a=dh.exec(e))&&(o=$m.exec(t));)(l=o.index)>s&&(l=t.slice(s,l),f[c]?f[c]+=l:f[++c]=l),(a=a[0])===(o=o[0])?f[c]?f[c]+=o:f[++c]=o:(f[++c]=null,m.push({i:c,x:Ks(a,o)})),s=$m.lastIndex;return s<t.length&&(l=t.slice(s),f[c]?f[c]+=l:f[++c]=l),f.length<2?m[0]?$$(m[0].x):D$(t):(t=m.length,function(p){for(var _=0,y;_<t;++_)f[(y=m[_]).i]=y.x(p);return f.join("")})}function $l(e,t){var s=typeof t,a;return t==null||s==="boolean"?Jd(t):(s==="number"?Ks:s==="string"?(a=co(t))?(t=a,bd):n1:t instanceof co?bd:t instanceof Date?M$:A$(t)?C$:Array.isArray(t)?O$:typeof t.valueOf!="function"&&typeof t.toString!="function"||isNaN(t)?z$:Ks)(e,t)}var A0=180/Math.PI,fh={translateX:0,translateY:0,rotate:0,skewX:0,scaleX:1,scaleY:1};function s1(e,t,s,a,o,l){var c,f,m;return(c=Math.sqrt(e*e+t*t))&&(e/=c,t/=c),(m=e*s+t*a)&&(s-=e*m,a-=t*m),(f=Math.sqrt(s*s+a*a))&&(s/=f,a/=f,m/=f),e*a<t*s&&(e=-e,t=-t,m=-m,c=-c),{translateX:o,translateY:l,rotate:Math.atan2(t,e)*A0,skewX:Math.atan(m)*A0,scaleX:c,scaleY:f}}var Lu;function R$(e){const t=new(typeof DOMMatrix=="function"?DOMMatrix:WebKitCSSMatrix)(e+"");return t.isIdentity?fh:s1(t.a,t.b,t.c,t.d,t.e,t.f)}function L$(e){return e==null||(Lu||(Lu=document.createElementNS("http://www.w3.org/2000/svg","g")),Lu.setAttribute("transform",e),!(e=Lu.transform.baseVal.consolidate()))?fh:(e=e.matrix,s1(e.a,e.b,e.c,e.d,e.e,e.f))}function i1(e,t,s,a){function o(p){return p.length?p.pop()+" ":""}function l(p,_,y,b,v,S){if(p!==y||_!==b){var w=v.push("translate(",null,t,null,s);S.push({i:w-4,x:Ks(p,y)},{i:w-2,x:Ks(_,b)})}else(y||b)&&v.push("translate("+y+t+b+s)}function c(p,_,y,b){p!==_?(p-_>180?_+=360:_-p>180&&(p+=360),b.push({i:y.push(o(y)+"rotate(",null,a)-2,x:Ks(p,_)})):_&&y.push(o(y)+"rotate("+_+a)}function f(p,_,y,b){p!==_?b.push({i:y.push(o(y)+"skewX(",null,a)-2,x:Ks(p,_)}):_&&y.push(o(y)+"skewX("+_+a)}function m(p,_,y,b,v,S){if(p!==y||_!==b){var w=v.push(o(v)+"scale(",null,",",null,")");S.push({i:w-4,x:Ks(p,y)},{i:w-2,x:Ks(_,b)})}else(y!==1||b!==1)&&v.push(o(v)+"scale("+y+","+b+")")}return function(p,_){var y=[],b=[];return p=e(p),_=e(_),l(p.translateX,p.translateY,_.translateX,_.translateY,y,b),c(p.rotate,_.rotate,y,b),f(p.skewX,_.skewX,y,b),m(p.scaleX,p.scaleY,_.scaleX,_.scaleY,y,b),p=_=null,function(v){for(var S=-1,w=b.length,k;++S<w;)y[(k=b[S]).i]=k.x(v);return y.join("")}}}var q$=i1(R$,"px, ","px)","deg)"),U$=i1(L$,", ",")",")"),B$=1e-12;function O0(e){return((e=Math.exp(e))+1/e)/2}function H$(e){return((e=Math.exp(e))-1/e)/2}function I$(e){return((e=Math.exp(2*e))-1)/(e+1)}var Xu=(function e(t,s,a){function o(l,c){var f=l[0],m=l[1],p=l[2],_=c[0],y=c[1],b=c[2],v=_-f,S=y-m,w=v*v+S*S,k,j;if(w<B$)j=Math.log(b/p)/t,k=function(R){return[f+R*v,m+R*S,p*Math.exp(t*R*j)]};else{var A=Math.sqrt(w),N=(b*b-p*p+a*w)/(2*p*s*A),T=(b*b-p*p-a*w)/(2*b*s*A),E=Math.log(Math.sqrt(N*N+1)-N),O=Math.log(Math.sqrt(T*T+1)-T);j=(O-E)/t,k=function(R){var q=R*j,Z=O0(E),oe=p/(s*A)*(Z*I$(t*q+E)-H$(E));return[f+oe*v,m+oe*S,p*Z/O0(t*q+E)]}}return k.duration=Math.abs(j)*1e3*t/Math.SQRT2,k}return o.rho=function(l){var c=Math.max(.001,+l),f=c*c,m=f*f;return e(c,f,m)},o})(Math.SQRT2,2,4);function a1(e){return(function t(s){s=+s;function a(o,l){var c=e((o=uh(o)).h,(l=uh(l)).h),f=yr(o.s,l.s),m=yr(o.l,l.l),p=yr(o.opacity,l.opacity);return function(_){return o.h=c(_),o.s=f(_),o.l=m(Math.pow(_,s)),o.opacity=p(_),o+""}}return a.gamma=t,a})(1)}a1(E$);a1(yr);var Sr=0,Al=0,Nl=0,o1=1e3,vd,Ol,xd=0,uo=0,Qd=0,Vl=typeof performance=="object"&&performance.now?performance:Date,r1=typeof window=="object"&&window.requestAnimationFrame?window.requestAnimationFrame.bind(window):function(e){setTimeout(e,17)};function og(){return uo||(r1(Z$),uo=Vl.now()+Qd)}function Z$(){uo=0}function wd(){this._call=this._time=this._next=null}wd.prototype=l1.prototype={constructor:wd,restart:function(e,t,s){if(typeof e!="function")throw new TypeError("callback is not a function");s=(s==null?og():+s)+(t==null?0:+t),!this._next&&Ol!==this&&(Ol?Ol._next=this:vd=this,Ol=this),this._call=e,this._time=s,ph()},stop:function(){this._call&&(this._call=null,this._time=1/0,ph())}};function l1(e,t,s){var a=new wd;return a.restart(e,t,s),a}function P$(){og(),++Sr;for(var e=vd,t;e;)(t=uo-e._time)>=0&&e._call.call(void 0,t),e=e._next;--Sr}function M0(){uo=(xd=Vl.now())+Qd,Sr=Al=0;try{P$()}finally{Sr=0,V$(),uo=0}}function Y$(){var e=Vl.now(),t=e-xd;t>o1&&(Qd-=t,xd=e)}function V$(){for(var e,t=vd,s,a=1/0;t;)t._call?(a>t._time&&(a=t._time),e=t,t=t._next):(s=t._next,t._next=null,t=e?e._next=s:vd=s);Ol=e,ph(a)}function ph(e){if(!Sr){Al&&(Al=clearTimeout(Al));var t=e-uo;t>24?(e<1/0&&(Al=setTimeout(M0,Math.ceil(e-Vl.now()-Qd)+1)),Nl&&(Nl=clearInterval(Nl))):(Nl||(xd=Vl.now(),Nl=setInterval(Y$,o1)),Sr=1,r1(M0))}}function z0(e,t,s){var a=new wd;return t=t==null?0:+t,a.restart(o=>{a.stop(),e(o+t)},t,s),a}function K$(e){return((e*=2)<=1?e*e*e:(e-=2)*e*e+2)/2}var rg=3;(function e(t){t=+t;function s(a){return Math.pow(a,t)}return s.exponent=e,s})(rg);(function e(t){t=+t;function s(a){return 1-Math.pow(1-a,t)}return s.exponent=e,s})(rg);(function e(t){t=+t;function s(a){return((a*=2)<=1?Math.pow(a,t):2-Math.pow(2-a,t))/2}return s.exponent=e,s})(rg);function Sd(e){return(Math.pow(2,-10*e)-.0009765625)*1.0009775171065494}var lg=1.70158;(function e(t){t=+t;function s(a){return(a=+a)*a*(t*(a-1)+a)}return s.overshoot=e,s})(lg);(function e(t){t=+t;function s(a){return--a*a*((a+1)*t+a)+1}return s.overshoot=e,s})(lg);(function e(t){t=+t;function s(a){return((a*=2)<1?a*a*((t+1)*a-t):(a-=2)*a*((t+1)*a+t)+2)/2}return s.overshoot=e,s})(lg);var kr=2*Math.PI,cg=1,ug=.3;(function e(t,s){var a=Math.asin(1/(t=Math.max(1,t)))*(s/=kr);function o(l){return t*Sd(- --l)*Math.sin((a-l)/s)}return o.amplitude=function(l){return e(l,s*kr)},o.period=function(l){return e(t,l)},o})(cg,ug);(function e(t,s){var a=Math.asin(1/(t=Math.max(1,t)))*(s/=kr);function o(l){return 1-t*Sd(l=+l)*Math.sin((l+a)/s)}return o.amplitude=function(l){return e(l,s*kr)},o.period=function(l){return e(t,l)},o})(cg,ug);(function e(t,s){var a=Math.asin(1/(t=Math.max(1,t)))*(s/=kr);function o(l){return((l=l*2-1)<0?t*Sd(-l)*Math.sin((a-l)/s):2-t*Sd(l)*Math.sin((a+l)/s))/2}return o.amplitude=function(l){return e(l,s*kr)},o.period=function(l){return e(t,l)},o})(cg,ug);var G$=Gd("start","end","cancel","interrupt"),X$=[],c1=0,D0=1,mh=2,Fu=3,$0=4,hh=5,Ju=6;function Wd(e,t,s,a,o,l){var c=e.__transition;if(!c)e.__transition={};else if(s in c)return;F$(e,s,{name:t,index:a,group:o,on:G$,tween:X$,time:l.time,delay:l.delay,duration:l.duration,ease:l.ease,timer:null,state:c1})}function dg(e,t){var s=Ls(e,t);if(s.state>c1)throw new Error("too late; already scheduled");return s}function Qs(e,t){var s=Ls(e,t);if(s.state>Fu)throw new Error("too late; already running");return s}function Ls(e,t){var s=e.__transition;if(!s||!(s=s[t]))throw new Error("transition not found");return s}function F$(e,t,s){var a=e.__transition,o;a[t]=s,s.timer=l1(l,0,s.time);function l(p){s.state=D0,s.timer.restart(c,s.delay,s.time),s.delay<=p&&c(p-s.delay)}function c(p){var _,y,b,v;if(s.state!==D0)return m();for(_ in a)if(v=a[_],v.name===s.name){if(v.state===Fu)return z0(c);v.state===$0?(v.state=Ju,v.timer.stop(),v.on.call("interrupt",e,e.__data__,v.index,v.group),delete a[_]):+_<t&&(v.state=Ju,v.timer.stop(),v.on.call("cancel",e,e.__data__,v.index,v.group),delete a[_])}if(z0(function(){s.state===Fu&&(s.state=$0,s.timer.restart(f,s.delay,s.time),f(p))}),s.state=mh,s.on.call("start",e,e.__data__,s.index,s.group),s.state===mh){for(s.state=Fu,o=new Array(b=s.tween.length),_=0,y=-1;_<b;++_)(v=s.tween[_].value.call(e,e.__data__,s.index,s.group))&&(o[++y]=v);o.length=y+1}}function f(p){for(var _=p<s.duration?s.ease.call(null,p/s.duration):(s.timer.restart(m),s.state=hh,1),y=-1,b=o.length;++y<b;)o[y].call(e,_);s.state===hh&&(s.on.call("end",e,e.__data__,s.index,s.group),m())}function m(){s.state=Ju,s.timer.stop(),delete a[t];for(var p in a)return;delete e.__transition}}function Qu(e,t){var s=e.__transition,a,o,l=!0,c;if(s){t=t==null?null:t+"";for(c in s){if((a=s[c]).name!==t){l=!1;continue}o=a.state>mh&&a.state<hh,a.state=Ju,a.timer.stop(),a.on.call(o?"interrupt":"cancel",e,e.__data__,a.index,a.group),delete s[c]}l&&delete e.__transition}}function J$(e){return this.each(function(){Qu(this,e)})}function Q$(e,t){var s,a;return function(){var o=Qs(this,e),l=o.tween;if(l!==s){a=s=l;for(var c=0,f=a.length;c<f;++c)if(a[c].name===t){a=a.slice(),a.splice(c,1);break}}o.tween=a}}function W$(e,t,s){var a,o;if(typeof s!="function")throw new Error;return function(){var l=Qs(this,e),c=l.tween;if(c!==a){o=(a=c).slice();for(var f={name:t,value:s},m=0,p=o.length;m<p;++m)if(o[m].name===t){o[m]=f;break}m===p&&o.push(f)}l.tween=o}}function eR(e,t){var s=this._id;if(e+="",arguments.length<2){for(var a=Ls(this.node(),s).tween,o=0,l=a.length,c;o<l;++o)if((c=a[o]).name===e)return c.value;return null}return this.each((t==null?Q$:W$)(s,e,t))}function fg(e,t,s){var a=e._id;return e.each(function(){var o=Qs(this,a);(o.value||(o.value={}))[t]=s.apply(this,arguments)}),function(o){return Ls(o,a).value[t]}}function u1(e,t){var s;return(typeof t=="number"?Ks:t instanceof co?bd:(s=co(t))?(t=s,bd):n1)(e,t)}function tR(e){return function(){this.removeAttribute(e)}}function nR(e){return function(){this.removeAttributeNS(e.space,e.local)}}function sR(e,t,s){var a,o=s+"",l;return function(){var c=this.getAttribute(e);return c===o?null:c===a?l:l=t(a=c,s)}}function iR(e,t,s){var a,o=s+"",l;return function(){var c=this.getAttributeNS(e.space,e.local);return c===o?null:c===a?l:l=t(a=c,s)}}function aR(e,t,s){var a,o,l;return function(){var c,f=s(this),m;return f==null?void this.removeAttribute(e):(c=this.getAttribute(e),m=f+"",c===m?null:c===a&&m===o?l:(o=m,l=t(a=c,f)))}}function oR(e,t,s){var a,o,l;return function(){var c,f=s(this),m;return f==null?void this.removeAttributeNS(e.space,e.local):(c=this.getAttributeNS(e.space,e.local),m=f+"",c===m?null:c===a&&m===o?l:(o=m,l=t(a=c,f)))}}function rR(e,t){var s=Xd(e),a=s==="transform"?U$:u1;return this.attrTween(e,typeof t=="function"?(s.local?oR:aR)(s,a,fg(this,"attr."+e,t)):t==null?(s.local?nR:tR)(s):(s.local?iR:sR)(s,a,t))}function lR(e,t){return function(s){this.setAttribute(e,t.call(this,s))}}function cR(e,t){return function(s){this.setAttributeNS(e.space,e.local,t.call(this,s))}}function uR(e,t){var s,a;function o(){var l=t.apply(this,arguments);return l!==a&&(s=(a=l)&&cR(e,l)),s}return o._value=t,o}function dR(e,t){var s,a;function o(){var l=t.apply(this,arguments);return l!==a&&(s=(a=l)&&lR(e,l)),s}return o._value=t,o}function fR(e,t){var s="attr."+e;if(arguments.length<2)return(s=this.tween(s))&&s._value;if(t==null)return this.tween(s,null);if(typeof t!="function")throw new Error;var a=Xd(e);return this.tween(s,(a.local?uR:dR)(a,t))}function pR(e,t){return function(){dg(this,e).delay=+t.apply(this,arguments)}}function mR(e,t){return t=+t,function(){dg(this,e).delay=t}}function hR(e){var t=this._id;return arguments.length?this.each((typeof e=="function"?pR:mR)(t,e)):Ls(this.node(),t).delay}function gR(e,t){return function(){Qs(this,e).duration=+t.apply(this,arguments)}}function yR(e,t){return t=+t,function(){Qs(this,e).duration=t}}function _R(e){var t=this._id;return arguments.length?this.each((typeof e=="function"?gR:yR)(t,e)):Ls(this.node(),t).duration}function bR(e,t){if(typeof t!="function")throw new Error;return function(){Qs(this,e).ease=t}}function vR(e){var t=this._id;return arguments.length?this.each(bR(t,e)):Ls(this.node(),t).ease}function xR(e,t){return function(){var s=t.apply(this,arguments);if(typeof s!="function")throw new Error;Qs(this,e).ease=s}}function wR(e){if(typeof e!="function")throw new Error;return this.each(xR(this._id,e))}function SR(e){typeof e!="function"&&(e=OS(e));for(var t=this._groups,s=t.length,a=new Array(s),o=0;o<s;++o)for(var l=t[o],c=l.length,f=a[o]=[],m,p=0;p<c;++p)(m=l[p])&&e.call(m,m.__data__,p,l)&&f.push(m);return new Mi(a,this._parents,this._name,this._id)}function kR(e){if(e._id!==this._id)throw new Error;for(var t=this._groups,s=e._groups,a=t.length,o=s.length,l=Math.min(a,o),c=new Array(a),f=0;f<l;++f)for(var m=t[f],p=s[f],_=m.length,y=c[f]=new Array(_),b,v=0;v<_;++v)(b=m[v]||p[v])&&(y[v]=b);for(;f<a;++f)c[f]=t[f];return new Mi(c,this._parents,this._name,this._id)}function jR(e){return(e+"").trim().split(/^|\s+/).every(function(t){var s=t.indexOf(".");return s>=0&&(t=t.slice(0,s)),!t||t==="start"})}function NR(e,t,s){var a,o,l=jR(t)?dg:Qs;return function(){var c=l(this,e),f=c.on;f!==a&&(o=(a=f).copy()).on(t,s),c.on=o}}function ER(e,t){var s=this._id;return arguments.length<2?Ls(this.node(),s).on.on(e):this.each(NR(s,e,t))}function TR(e){return function(){var t=this.parentNode;for(var s in this.__transition)if(+s!==e)return;t&&t.removeChild(this)}}function CR(){return this.on("end.remove",TR(this._id))}function AR(e){var t=this._name,s=this._id;typeof e!="function"&&(e=tg(e));for(var a=this._groups,o=a.length,l=new Array(o),c=0;c<o;++c)for(var f=a[c],m=f.length,p=l[c]=new Array(m),_,y,b=0;b<m;++b)(_=f[b])&&(y=e.call(_,_.__data__,b,f))&&("__data__"in _&&(y.__data__=_.__data__),p[b]=y,Wd(p[b],t,s,b,p,Ls(_,s)));return new Mi(l,this._parents,t,s)}function OR(e){var t=this._name,s=this._id;typeof e!="function"&&(e=AS(e));for(var a=this._groups,o=a.length,l=[],c=[],f=0;f<o;++f)for(var m=a[f],p=m.length,_,y=0;y<p;++y)if(_=m[y]){for(var b=e.call(_,_.__data__,y,m),v,S=Ls(_,s),w=0,k=b.length;w<k;++w)(v=b[w])&&Wd(v,t,s,w,b,S);l.push(b),c.push(_)}return new Mi(l,c,t,s)}var MR=rc.prototype.constructor;function zR(){return new MR(this._groups,this._parents)}function DR(e,t){var s,a,o;return function(){var l=xr(this,e),c=(this.style.removeProperty(e),xr(this,e));return l===c?null:l===s&&c===a?o:o=t(s=l,a=c)}}function d1(e){return function(){this.style.removeProperty(e)}}function $R(e,t,s){var a,o=s+"",l;return function(){var c=xr(this,e);return c===o?null:c===a?l:l=t(a=c,s)}}function RR(e,t,s){var a,o,l;return function(){var c=xr(this,e),f=s(this),m=f+"";return f==null&&(m=f=(this.style.removeProperty(e),xr(this,e))),c===m?null:c===a&&m===o?l:(o=m,l=t(a=c,f))}}function LR(e,t){var s,a,o,l="style."+t,c="end."+l,f;return function(){var m=Qs(this,e),p=m.on,_=m.value[l]==null?f||(f=d1(t)):void 0;(p!==s||o!==_)&&(a=(s=p).copy()).on(c,o=_),m.on=a}}function qR(e,t,s){var a=(e+="")=="transform"?q$:u1;return t==null?this.styleTween(e,DR(e,a)).on("end.style."+e,d1(e)):typeof t=="function"?this.styleTween(e,RR(e,a,fg(this,"style."+e,t))).each(LR(this._id,e)):this.styleTween(e,$R(e,a,t),s).on("end.style."+e,null)}function UR(e,t,s){return function(a){this.style.setProperty(e,t.call(this,a),s)}}function BR(e,t,s){var a,o;function l(){var c=t.apply(this,arguments);return c!==o&&(a=(o=c)&&UR(e,c,s)),a}return l._value=t,l}function HR(e,t,s){var a="style."+(e+="");if(arguments.length<2)return(a=this.tween(a))&&a._value;if(t==null)return this.tween(a,null);if(typeof t!="function")throw new Error;return this.tween(a,BR(e,t,s??""))}function IR(e){return function(){this.textContent=e}}function ZR(e){return function(){var t=e(this);this.textContent=t??""}}function PR(e){return this.tween("text",typeof e=="function"?ZR(fg(this,"text",e)):IR(e==null?"":e+""))}function YR(e){return function(t){this.textContent=e.call(this,t)}}function VR(e){var t,s;function a(){var o=e.apply(this,arguments);return o!==s&&(t=(s=o)&&YR(o)),t}return a._value=e,a}function KR(e){var t="text";if(arguments.length<1)return(t=this.tween(t))&&t._value;if(e==null)return this.tween(t,null);if(typeof e!="function")throw new Error;return this.tween(t,VR(e))}function GR(){for(var e=this._name,t=this._id,s=f1(),a=this._groups,o=a.length,l=0;l<o;++l)for(var c=a[l],f=c.length,m,p=0;p<f;++p)if(m=c[p]){var _=Ls(m,t);Wd(m,e,s,p,c,{time:_.time+_.delay+_.duration,delay:0,duration:_.duration,ease:_.ease})}return new Mi(a,this._parents,e,s)}function XR(){var e,t,s=this,a=s._id,o=s.size();return new Promise(function(l,c){var f={value:c},m={value:function(){--o===0&&l()}};s.each(function(){var p=Qs(this,a),_=p.on;_!==e&&(t=(e=_).copy(),t._.cancel.push(f),t._.interrupt.push(f),t._.end.push(m)),p.on=t}),o===0&&l()})}var FR=0;function Mi(e,t,s,a){this._groups=e,this._parents=t,this._name=s,this._id=a}function f1(){return++FR}var wi=rc.prototype;Mi.prototype={constructor:Mi,select:AR,selectAll:OR,selectChild:wi.selectChild,selectChildren:wi.selectChildren,filter:SR,merge:kR,selection:zR,transition:GR,call:wi.call,nodes:wi.nodes,node:wi.node,size:wi.size,empty:wi.empty,each:wi.each,on:ER,attr:rR,attrTween:fR,style:qR,styleTween:HR,text:PR,textTween:KR,remove:CR,tween:eR,delay:hR,duration:_R,ease:vR,easeVarying:wR,end:XR,[Symbol.iterator]:wi[Symbol.iterator]};var JR={time:null,delay:0,duration:250,ease:K$};function QR(e,t){for(var s;!(s=e.__transition)||!(s=s[t]);)if(!(e=e.parentNode))throw new Error(`transition ${t} not found`);return s}function WR(e){var t,s;e instanceof Mi?(t=e._id,e=e._name):(t=f1(),(s=JR).time=og(),e=e==null?null:e+"");for(var a=this._groups,o=a.length,l=0;l<o;++l)for(var c=a[l],f=c.length,m,p=0;p<f;++p)(m=c[p])&&Wd(m,e,t,p,c,s||QR(m,t));return new Mi(a,this._parents,e,t)}rc.prototype.interrupt=J$;rc.prototype.transition=WR;var qu=e=>()=>e;function e3(e,{sourceEvent:t,target:s,transform:a,dispatch:o}){Object.defineProperties(this,{type:{value:e,enumerable:!0,configurable:!0},sourceEvent:{value:t,enumerable:!0,configurable:!0},target:{value:s,enumerable:!0,configurable:!0},transform:{value:a,enumerable:!0,configurable:!0},_:{value:o}})}function ji(e,t,s){this.k=e,this.x=t,this.y=s}ji.prototype={constructor:ji,scale:function(e){return e===1?this:new ji(this.k*e,this.x,this.y)},translate:function(e,t){return e===0&t===0?this:new ji(this.k,this.x+this.k*e,this.y+this.k*t)},apply:function(e){return[e[0]*this.k+this.x,e[1]*this.k+this.y]},applyX:function(e){return e*this.k+this.x},applyY:function(e){return e*this.k+this.y},invert:function(e){return[(e[0]-this.x)/this.k,(e[1]-this.y)/this.k]},invertX:function(e){return(e-this.x)/this.k},invertY:function(e){return(e-this.y)/this.k},rescaleX:function(e){return e.copy().domain(e.range().map(this.invertX,this).map(e.invert,e))},rescaleY:function(e){return e.copy().domain(e.range().map(this.invertY,this).map(e.invert,e))},toString:function(){return"translate("+this.x+","+this.y+") scale("+this.k+")"}};var ef=new ji(1,0,0);p1.prototype=ji.prototype;function p1(e){for(;!e.__zoom;)if(!(e=e.parentNode))return ef;return e.__zoom}function Rm(e){e.stopImmediatePropagation()}function El(e){e.preventDefault(),e.stopImmediatePropagation()}function t3(e){return(!e.ctrlKey||e.type==="wheel")&&!e.button}function n3(){var e=this;return e instanceof SVGElement?(e=e.ownerSVGElement||e,e.hasAttribute("viewBox")?(e=e.viewBox.baseVal,[[e.x,e.y],[e.x+e.width,e.y+e.height]]):[[0,0],[e.width.baseVal.value,e.height.baseVal.value]]):[[0,0],[e.clientWidth,e.clientHeight]]}function R0(){return this.__zoom||ef}function s3(e){return-e.deltaY*(e.deltaMode===1?.05:e.deltaMode?1:.002)*(e.ctrlKey?10:1)}function i3(){return navigator.maxTouchPoints||"ontouchstart"in this}function a3(e,t,s){var a=e.invertX(t[0][0])-s[0][0],o=e.invertX(t[1][0])-s[1][0],l=e.invertY(t[0][1])-s[0][1],c=e.invertY(t[1][1])-s[1][1];return e.translate(o>a?(a+o)/2:Math.min(0,a)||Math.max(0,o),c>l?(l+c)/2:Math.min(0,l)||Math.max(0,c))}function m1(){var e=t3,t=n3,s=a3,a=s3,o=i3,l=[0,1/0],c=[[-1/0,-1/0],[1/0,1/0]],f=250,m=Xu,p=Gd("start","zoom","end"),_,y,b,v=500,S=150,w=0,k=10;function j(L){L.property("__zoom",R0).on("wheel.zoom",q,{passive:!1}).on("mousedown.zoom",Z).on("dblclick.zoom",oe).filter(o).on("touchstart.zoom",Q).on("touchmove.zoom",ee).on("touchend.zoom touchcancel.zoom",X).style("-webkit-tap-highlight-color","rgba(0,0,0,0)")}j.transform=function(L,I,D,U){var B=L.selection?L.selection():L;B.property("__zoom",R0),L!==B?E(L,I,D,U):B.interrupt().each(function(){O(this,arguments).event(U).start().zoom(null,typeof I=="function"?I.apply(this,arguments):I).end()})},j.scaleBy=function(L,I,D,U){j.scaleTo(L,function(){var B=this.__zoom.k,V=typeof I=="function"?I.apply(this,arguments):I;return B*V},D,U)},j.scaleTo=function(L,I,D,U){j.transform(L,function(){var B=t.apply(this,arguments),V=this.__zoom,te=D==null?T(B):typeof D=="function"?D.apply(this,arguments):D,z=V.invert(te),Y=typeof I=="function"?I.apply(this,arguments):I;return s(N(A(V,Y),te,z),B,c)},D,U)},j.translateBy=function(L,I,D,U){j.transform(L,function(){return s(this.__zoom.translate(typeof I=="function"?I.apply(this,arguments):I,typeof D=="function"?D.apply(this,arguments):D),t.apply(this,arguments),c)},null,U)},j.translateTo=function(L,I,D,U,B){j.transform(L,function(){var V=t.apply(this,arguments),te=this.__zoom,z=U==null?T(V):typeof U=="function"?U.apply(this,arguments):U;return s(ef.translate(z[0],z[1]).scale(te.k).translate(typeof I=="function"?-I.apply(this,arguments):-I,typeof D=="function"?-D.apply(this,arguments):-D),V,c)},U,B)};function A(L,I){return I=Math.max(l[0],Math.min(l[1],I)),I===L.k?L:new ji(I,L.x,L.y)}function N(L,I,D){var U=I[0]-D[0]*L.k,B=I[1]-D[1]*L.k;return U===L.x&&B===L.y?L:new ji(L.k,U,B)}function T(L){return[(+L[0][0]+ +L[1][0])/2,(+L[0][1]+ +L[1][1])/2]}function E(L,I,D,U){L.on("start.zoom",function(){O(this,arguments).event(U).start()}).on("interrupt.zoom end.zoom",function(){O(this,arguments).event(U).end()}).tween("zoom",function(){var B=this,V=arguments,te=O(B,V).event(U),z=t.apply(B,V),Y=D==null?T(z):typeof D=="function"?D.apply(B,V):D,ce=Math.max(z[1][0]-z[0][0],z[1][1]-z[0][1]),de=B.__zoom,_e=typeof I=="function"?I.apply(B,V):I,F=m(de.invert(Y).concat(ce/de.k),_e.invert(Y).concat(ce/_e.k));return function(G){if(G===1)G=_e;else{var P=F(G),ae=ce/P[2];G=new ji(ae,Y[0]-P[0]*ae,Y[1]-P[1]*ae)}te.zoom(null,G)}})}function O(L,I,D){return!D&&L.__zooming||new R(L,I)}function R(L,I){this.that=L,this.args=I,this.active=0,this.sourceEvent=null,this.extent=t.apply(L,I),this.taps=0}R.prototype={event:function(L){return L&&(this.sourceEvent=L),this},start:function(){return++this.active===1&&(this.that.__zooming=this,this.emit("start")),this},zoom:function(L,I){return this.mouse&&L!=="mouse"&&(this.mouse[1]=I.invert(this.mouse[0])),this.touch0&&L!=="touch"&&(this.touch0[1]=I.invert(this.touch0[0])),this.touch1&&L!=="touch"&&(this.touch1[1]=I.invert(this.touch1[0])),this.that.__zoom=I,this.emit("zoom"),this},end:function(){return--this.active===0&&(delete this.that.__zooming,this.emit("end")),this},emit:function(L){var I=ss(this.that).datum();p.call(L,this.that,new e3(L,{sourceEvent:this.sourceEvent,target:j,transform:this.that.__zoom,dispatch:p}),I)}};function q(L,...I){if(!e.apply(this,arguments))return;var D=O(this,I).event(L),U=this.__zoom,B=Math.max(l[0],Math.min(l[1],U.k*Math.pow(2,a.apply(this,arguments)))),V=Cs(L);if(D.wheel)(D.mouse[0][0]!==V[0]||D.mouse[0][1]!==V[1])&&(D.mouse[1]=U.invert(D.mouse[0]=V)),clearTimeout(D.wheel);else{if(U.k===B)return;D.mouse=[V,U.invert(V)],Qu(this),D.start()}El(L),D.wheel=setTimeout(te,S),D.zoom("mouse",s(N(A(U,B),D.mouse[0],D.mouse[1]),D.extent,c));function te(){D.wheel=null,D.end()}}function Z(L,...I){if(b||!e.apply(this,arguments))return;var D=L.currentTarget,U=O(this,I,!0).event(L),B=ss(L.view).on("mousemove.zoom",Y,!0).on("mouseup.zoom",ce,!0),V=Cs(L,D),te=L.clientX,z=L.clientY;HS(L.view),Rm(L),U.mouse=[V,this.__zoom.invert(V)],Qu(this),U.start();function Y(de){if(El(de),!U.moved){var _e=de.clientX-te,F=de.clientY-z;U.moved=_e*_e+F*F>w}U.event(de).zoom("mouse",s(N(U.that.__zoom,U.mouse[0]=Cs(de,D),U.mouse[1]),U.extent,c))}function ce(de){B.on("mousemove.zoom mouseup.zoom",null),IS(de.view,U.moved),El(de),U.event(de).end()}}function oe(L,...I){if(e.apply(this,arguments)){var D=this.__zoom,U=Cs(L.changedTouches?L.changedTouches[0]:L,this),B=D.invert(U),V=D.k*(L.shiftKey?.5:2),te=s(N(A(D,V),U,B),t.apply(this,I),c);El(L),f>0?ss(this).transition().duration(f).call(E,te,U,L):ss(this).call(j.transform,te,U,L)}}function Q(L,...I){if(e.apply(this,arguments)){var D=L.touches,U=D.length,B=O(this,I,L.changedTouches.length===U).event(L),V,te,z,Y;for(Rm(L),te=0;te<U;++te)z=D[te],Y=Cs(z,this),Y=[Y,this.__zoom.invert(Y),z.identifier],B.touch0?!B.touch1&&B.touch0[2]!==Y[2]&&(B.touch1=Y,B.taps=0):(B.touch0=Y,V=!0,B.taps=1+!!_);_&&(_=clearTimeout(_)),V&&(B.taps<2&&(y=Y[0],_=setTimeout(function(){_=null},v)),Qu(this),B.start())}}function ee(L,...I){if(this.__zooming){var D=O(this,I).event(L),U=L.changedTouches,B=U.length,V,te,z,Y;for(El(L),V=0;V<B;++V)te=U[V],z=Cs(te,this),D.touch0&&D.touch0[2]===te.identifier?D.touch0[0]=z:D.touch1&&D.touch1[2]===te.identifier&&(D.touch1[0]=z);if(te=D.that.__zoom,D.touch1){var ce=D.touch0[0],de=D.touch0[1],_e=D.touch1[0],F=D.touch1[1],G=(G=_e[0]-ce[0])*G+(G=_e[1]-ce[1])*G,P=(P=F[0]-de[0])*P+(P=F[1]-de[1])*P;te=A(te,Math.sqrt(G/P)),z=[(ce[0]+_e[0])/2,(ce[1]+_e[1])/2],Y=[(de[0]+F[0])/2,(de[1]+F[1])/2]}else if(D.touch0)z=D.touch0[0],Y=D.touch0[1];else return;D.zoom("touch",s(N(te,z,Y),D.extent,c))}}function X(L,...I){if(this.__zooming){var D=O(this,I).event(L),U=L.changedTouches,B=U.length,V,te;for(Rm(L),b&&clearTimeout(b),b=setTimeout(function(){b=null},v),V=0;V<B;++V)te=U[V],D.touch0&&D.touch0[2]===te.identifier?delete D.touch0:D.touch1&&D.touch1[2]===te.identifier&&delete D.touch1;if(D.touch1&&!D.touch0&&(D.touch0=D.touch1,delete D.touch1),D.touch0)D.touch0[1]=this.__zoom.invert(D.touch0[0]);else if(D.end(),D.taps===2&&(te=Cs(te,this),Math.hypot(y[0]-te[0],y[1]-te[1])<k)){var z=ss(this).on("dblclick.zoom");z&&z.apply(this,arguments)}}}return j.wheelDelta=function(L){return arguments.length?(a=typeof L=="function"?L:qu(+L),j):a},j.filter=function(L){return arguments.length?(e=typeof L=="function"?L:qu(!!L),j):e},j.touchable=function(L){return arguments.length?(o=typeof L=="function"?L:qu(!!L),j):o},j.extent=function(L){return arguments.length?(t=typeof L=="function"?L:qu([[+L[0][0],+L[0][1]],[+L[1][0],+L[1][1]]]),j):t},j.scaleExtent=function(L){return arguments.length?(l[0]=+L[0],l[1]=+L[1],j):[l[0],l[1]]},j.translateExtent=function(L){return arguments.length?(c[0][0]=+L[0][0],c[1][0]=+L[1][0],c[0][1]=+L[0][1],c[1][1]=+L[1][1],j):[[c[0][0],c[0][1]],[c[1][0],c[1][1]]]},j.constrain=function(L){return arguments.length?(s=L,j):s},j.duration=function(L){return arguments.length?(f=+L,j):f},j.interpolate=function(L){return arguments.length?(m=L,j):m},j.on=function(){var L=p.on.apply(p,arguments);return L===p?j:L},j.clickDistance=function(L){return arguments.length?(w=(L=+L)*L,j):Math.sqrt(w)},j.tapDistance=function(L){return arguments.length?(k=+L,j):k},j}const Ds={error001:(e="react")=>`Seems like you have not used ${e==="svelte"?"SvelteFlowProvider":"ReactFlowProvider"} as an ancestor. Help: https://${e}flow.dev/error#001`,error002:()=>"It looks like you've created a new nodeTypes or edgeTypes object. If this wasn't on purpose please define the nodeTypes/edgeTypes outside of the component or memoize them.",error003:e=>`Node type "${e}" not found. Using fallback type "default".`,error004:()=>"The parent container needs a width and a height to render the graph.",error005:()=>"Only child nodes can use a parent extent.",error006:()=>"Can't create edge. An edge needs a source and a target.",error007:e=>`The old edge with id=${e} does not exist.`,error009:e=>`Marker type "${e}" doesn't exist.`,error008:(e,{id:t,sourceHandle:s,targetHandle:a})=>`Couldn't create edge for ${e} handle id: "${e==="source"?s:a}", edge id: ${t}.`,error010:()=>"Handle: No node id found. Make sure to only use a Handle inside a custom Node.",error011:e=>`Edge type "${e}" not found. Using fallback type "default".`,error012:e=>`Node with id "${e}" does not exist, it may have been removed. This can happen when a node is deleted before the "onNodeClick" handler is called.`,error013:(e="react")=>`It seems that you haven't loaded the styles. Please import '@xyflow/${e}/dist/style.css' or base.css to make sure everything is working properly.`,error014:()=>"useNodeConnections: No node ID found. Call useNodeConnections inside a custom Node or provide a node ID.",error015:()=>"It seems that you are trying to drag a node that is not initialized. Please use onNodesChange as explained in the docs.",error016:e=>`Edge with id "${e}" does not exist, it may have been removed. This can happen when an edge is deleted before the "onEdgeClick" handler is called.`},Kl=[[Number.NEGATIVE_INFINITY,Number.NEGATIVE_INFINITY],[Number.POSITIVE_INFINITY,Number.POSITIVE_INFINITY]],h1=["Enter"," ","Escape"],g1={"node.a11yDescription.default":"Press enter or space to select a node. Press delete to remove it and escape to cancel.","node.a11yDescription.keyboardDisabled":"Press enter or space to select a node. You can then use the arrow keys to move the node around. Press delete to remove it and escape to cancel.","node.a11yDescription.ariaLiveMessage":({direction:e,x:t,y:s})=>`Moved selected node ${e}. New position, x: ${t}, y: ${s}`,"edge.a11yDescription.default":"Press enter or space to select an edge. You can then press delete to remove it or escape to cancel.","controls.ariaLabel":"Control Panel","controls.zoomIn.ariaLabel":"Zoom In","controls.zoomOut.ariaLabel":"Zoom Out","controls.fitView.ariaLabel":"Fit View","controls.interactive.ariaLabel":"Toggle Interactivity","minimap.ariaLabel":"Mini Map","handle.ariaLabel":"Handle"};var jr;(function(e){e.Strict="strict",e.Loose="loose"})(jr||(jr={}));var so;(function(e){e.Free="free",e.Vertical="vertical",e.Horizontal="horizontal"})(so||(so={}));var Gl;(function(e){e.Partial="partial",e.Full="full"})(Gl||(Gl={}));const y1={inProgress:!1,isValid:null,from:null,fromHandle:null,fromPosition:null,fromNode:null,to:null,toHandle:null,toPosition:null,toNode:null,pointer:null};var pa;(function(e){e.Bezier="default",e.Straight="straight",e.Step="step",e.SmoothStep="smoothstep",e.SimpleBezier="simplebezier"})(pa||(pa={}));var Xl;(function(e){e.Arrow="arrow",e.ArrowClosed="arrowclosed"})(Xl||(Xl={}));var ze;(function(e){e.Left="left",e.Top="top",e.Right="right",e.Bottom="bottom"})(ze||(ze={}));const L0={[ze.Left]:ze.Right,[ze.Right]:ze.Left,[ze.Top]:ze.Bottom,[ze.Bottom]:ze.Top},_1=e=>!!e&&typeof e=="object"&&"id"in e&&"source"in e&&"target"in e,o3=e=>!!e&&typeof e=="object"&&"id"in e&&"position"in e&&!("source"in e)&&!("target"in e),pg=e=>!!e&&typeof e=="object"&&"id"in e&&"internals"in e&&!("source"in e)&&!("target"in e),cc=(e,t=[0,0])=>{const{width:s,height:a}=qs(e),o=e.origin??t,l=s*o[0],c=a*o[1];return{x:e.position.x-l,y:e.position.y-c}},r3=(e,t={nodeOrigin:[0,0]})=>{if(e.length===0)return{x:0,y:0,width:0,height:0};let s=!1;const a=e.reduce((o,l)=>{const c=typeof l=="string";let f=!t.nodeLookup&&!c?l:void 0;return t.nodeLookup&&(f=c?t.nodeLookup.get(l):pg(l)?l:t.nodeLookup.get(l.id)),f?(s=!0,tf(o,kd(f,t.nodeOrigin))):o},{x:1/0,y:1/0,x2:-1/0,y2:-1/0});return s?nf(a):{x:0,y:0,width:0,height:0}},uc=(e,t={})=>{let s={x:1/0,y:1/0,x2:-1/0,y2:-1/0},a=!1;return e.forEach(o=>{(t.filter===void 0||t.filter(o))&&(s=tf(s,kd(o)),a=!0)}),a?nf(s):{x:0,y:0,width:0,height:0}},mg=(e,t,[s,a,o]=[0,0,1],l=!1,c=!1)=>{const f=(t.x-s)/o,m=(t.y-a)/o,p=t.width/o,_=t.height/o,y=[];for(const b of e.values()){const{measured:v,selectable:S=!0,hidden:w=!1}=b;if(c&&!S||w)continue;const k=v.width??b.width??b.initialWidth??0,j=v.height??b.height??b.initialHeight??0,{x:A,y:N}=b.internals.positionAbsolute,T=w1(f,m,p,_,A,N,k,j),E=k*j,O=l&&T>0;(!b.internals.handleBounds||O||T>=E||b.dragging)&&y.push(b)}return y},l3=(e,t)=>{const s=new Set;return e.forEach(a=>{s.add(a.id)}),t.filter(a=>s.has(a.source)||s.has(a.target))};function c3(e,t){const s=new Map,a=t?.nodes?new Set(t.nodes.map(o=>o.id)):null;return e.forEach(o=>{let l;if(t?.includeHiddenNodes){const{width:c,height:f}=qs(o);l=c>0&&f>0}else l=!!(o.measured.width&&o.measured.height&&!o.hidden);l&&(!a||a.has(o.id))&&s.set(o.id,o)}),s}async function u3({nodes:e,width:t,height:s,panZoom:a,minZoom:o,maxZoom:l},c){if(e.size===0)return!0;const f=c3(e,c),m=uc(f),p=gg(m,t,s,c?.minZoom??o,c?.maxZoom??l,c?.padding??.1);return await a.setViewport(p,{duration:c?.duration,ease:c?.ease,interpolate:c?.interpolate}),!0}function b1({nodeId:e,nextPosition:t,nodeLookup:s,nodeOrigin:a=[0,0],nodeExtent:o,onError:l}){const c=s.get(e),f=c.parentId?s.get(c.parentId):void 0,{x:m,y:p}=f?f.internals.positionAbsolute:{x:0,y:0},_=c.origin??a;let y=c.extent||o;if(c.extent==="parent"&&!c.expandParent)if(!f)l?.("005",Ds.error005());else{const{width:v,height:S}=qs(f);v&&S&&(y=[[m,p],[m+v,p+S]])}else f&&po(c.extent)&&(y=[[c.extent[0][0]+m,c.extent[0][1]+p],[c.extent[1][0]+m,c.extent[1][1]+p]]);const b=po(y)?fo(t,y,c.measured):t;return(c.measured.width===void 0||c.measured.height===void 0)&&l?.("015",Ds.error015()),{position:{x:b.x-m+(c.measured.width??0)*_[0],y:b.y-p+(c.measured.height??0)*_[1]},positionAbsolute:b}}async function d3({nodesToRemove:e=[],edgesToRemove:t=[],nodes:s,edges:a,onBeforeDelete:o}){const l=new Set(e.map(b=>b.id)),c=[];for(const b of s){if(b.deletable===!1)continue;const v=l.has(b.id),S=!v&&b.parentId&&c.find(w=>w.id===b.parentId);(v||S)&&c.push(b)}const f=new Set(t.map(b=>b.id)),m=a.filter(b=>b.deletable!==!1),_=l3(c,m);for(const b of m)f.has(b.id)&&!_.find(S=>S.id===b.id)&&_.push(b);if(!o)return{edges:_,nodes:c};const y=await o({nodes:c,edges:_});return typeof y=="boolean"?y?{edges:_,nodes:c}:{edges:[],nodes:[]}:y}const Nr=(e,t=0,s=1)=>Math.min(Math.max(e,t),s),fo=(e={x:0,y:0},t,s)=>({x:Nr(e.x,t[0][0],t[1][0]-(s?.width??0)),y:Nr(e.y,t[0][1],t[1][1]-(s?.height??0))});function v1(e,t,s){const{width:a,height:o}=qs(s),{x:l,y:c}=s.internals.positionAbsolute;return fo(e,[[l,c],[l+a,c+o]],t)}const q0=(e,t,s)=>e<t?Nr(Math.abs(e-t),1,t)/t:e>s?-Nr(Math.abs(e-s),1,t)/t:0,hg=(e,t,s=15,a=40)=>{const o=q0(e.x,a,t.width-a)*s,l=q0(e.y,a,t.height-a)*s;return[o,l]},tf=(e,t)=>({x:Math.min(e.x,t.x),y:Math.min(e.y,t.y),x2:Math.max(e.x2,t.x2),y2:Math.max(e.y2,t.y2)}),gh=({x:e,y:t,width:s,height:a})=>({x:e,y:t,x2:e+s,y2:t+a}),nf=({x:e,y:t,x2:s,y2:a})=>({x:e,y:t,width:s-e,height:a-t}),Fl=(e,t=[0,0])=>{const{x:s,y:a}=pg(e)?e.internals.positionAbsolute:cc(e,t);return{x:s,y:a,width:e.measured?.width??e.width??e.initialWidth??0,height:e.measured?.height??e.height??e.initialHeight??0}},kd=(e,t=[0,0])=>{const{x:s,y:a}=pg(e)?e.internals.positionAbsolute:cc(e,t);return{x:s,y:a,x2:s+(e.measured?.width??e.width??e.initialWidth??0),y2:a+(e.measured?.height??e.height??e.initialHeight??0)}},x1=(e,t)=>nf(tf(gh(e),gh(t))),w1=(e,t,s,a,o,l,c,f)=>{const m=Math.max(0,Math.min(e+s,o+c)-Math.max(e,o)),p=Math.max(0,Math.min(t+a,l+f)-Math.max(t,l));return Math.ceil(m*p)},jd=(e,t)=>w1(e.x,e.y,e.width,e.height,t.x,t.y,t.width,t.height),U0=e=>Ms(e.width)&&Ms(e.height)&&Ms(e.x)&&Ms(e.y),Ms=e=>!isNaN(e)&&isFinite(e),S1=(e,t)=>(s,a)=>{},dc=(e,t=[1,1])=>({x:t[0]*Math.round(e.x/t[0]),y:t[1]*Math.round(e.y/t[1])}),fc=({x:e,y:t},[s,a,o],l=!1,c=[1,1])=>{const f={x:(e-s)/o,y:(t-a)/o};return l?dc(f,c):f},Er=({x:e,y:t},[s,a,o])=>({x:e*o+s,y:t*o+a});function tr(e,t){if(typeof e=="number")return Math.floor((t-t/(1+e))*.5);if(typeof e=="string"&&e.endsWith("px")){const s=parseFloat(e);if(!Number.isNaN(s))return Math.floor(s)}if(typeof e=="string"&&e.endsWith("%")){const s=parseFloat(e);if(!Number.isNaN(s))return Math.floor(t*s*.01)}return console.error(`The padding value "${e}" is invalid. Please provide a number or a string with a valid unit (px or %).`),0}function f3(e,t,s){if(typeof e=="string"||typeof e=="number"){const a=tr(e,s),o=tr(e,t);return{top:a,right:o,bottom:a,left:o,x:o*2,y:a*2}}if(typeof e=="object"){const a=tr(e.top??e.y??0,s),o=tr(e.bottom??e.y??0,s),l=tr(e.left??e.x??0,t),c=tr(e.right??e.x??0,t);return{top:a,right:c,bottom:o,left:l,x:l+c,y:a+o}}return{top:0,right:0,bottom:0,left:0,x:0,y:0}}function p3(e,t,s,a,o,l){const{x:c,y:f}=Er(e,[t,s,a]),{x:m,y:p}=Er({x:e.x+e.width,y:e.y+e.height},[t,s,a]),_=o-m,y=l-p;return{left:Math.floor(c),top:Math.floor(f),right:Math.floor(_),bottom:Math.floor(y)}}const gg=(e,t,s,a,o,l)=>{const c=f3(l,t,s),f=(t-c.x)/e.width,m=(s-c.y)/e.height,p=Math.min(f,m),_=Nr(p,a,o),y=e.x+e.width/2,b=e.y+e.height/2,v=t/2-y*_,S=s/2-b*_,w=p3(e,v,S,_,t,s),k={left:Math.min(w.left-c.left,0),top:Math.min(w.top-c.top,0),right:Math.min(w.right-c.right,0),bottom:Math.min(w.bottom-c.bottom,0)};return{x:v-k.left+k.right,y:S-k.top+k.bottom,zoom:_}},Jl=()=>typeof navigator<"u"&&navigator?.userAgent?.indexOf("Mac")>=0;function po(e){return e!=null&&e!=="parent"}function qs(e){return{width:e.measured?.width??e.width??e.initialWidth??0,height:e.measured?.height??e.height??e.initialHeight??0}}function k1(e){return(e.measured?.width??e.width??e.initialWidth)!==void 0&&(e.measured?.height??e.height??e.initialHeight)!==void 0}function j1(e,t={width:0,height:0},s,a,o){const l={...e},c=a.get(s);if(c){const f=c.origin||o;l.x+=c.internals.positionAbsolute.x-(t.width??0)*f[0],l.y+=c.internals.positionAbsolute.y-(t.height??0)*f[1]}return l}function B0(e,t){if(e.size!==t.size)return!1;for(const s of e)if(!t.has(s))return!1;return!0}function m3(){let e,t;return{promise:new Promise((a,o)=>{e=a,t=o}),resolve:e,reject:t}}function h3(e){return{...g1,...e||{}}}function N1(e){return e===null?null:e?"valid":"invalid"}function Rl(e,{snapGrid:t=[0,0],snapToGrid:s=!1,transform:a,containerBounds:o}){const{x:l,y:c}=zs(e),f=fc({x:l-(o?.left??0),y:c-(o?.top??0)},a),{x:m,y:p}=s?dc(f,t):f;return{xSnapped:m,ySnapped:p,...f}}const yg=e=>({width:e.offsetWidth,height:e.offsetHeight}),E1=e=>e?.getRootNode?.()||window?.document,g3=["INPUT","SELECT","TEXTAREA"];function T1(e){const t=e.composedPath?.()?.[0]||e.target;return t?.nodeType!==1?!1:g3.includes(t.nodeName)||t.hasAttribute("contenteditable")||!!t.closest(".nokey")}const C1=e=>"clientX"in e,zs=(e,t)=>{const s=C1(e),a=s?e.clientX:e.touches?.[0].clientX,o=s?e.clientY:e.touches?.[0].clientY;return{x:a-(t?.left??0),y:o-(t?.top??0)}},H0=(e,t,s,a,o)=>{const l=t.querySelectorAll(`.${e}`);return!l||!l.length?null:Array.from(l).map(c=>{const f=c.getBoundingClientRect();return{id:c.getAttribute("data-handleid"),type:e,nodeId:o,position:c.getAttribute("data-handlepos"),x:(f.left-s.left)/a,y:(f.top-s.top)/a,...yg(c)}})};function A1({sourceX:e,sourceY:t,targetX:s,targetY:a,sourceControlX:o,sourceControlY:l,targetControlX:c,targetControlY:f}){const m=e*.125+o*.375+c*.375+s*.125,p=t*.125+l*.375+f*.375+a*.125,_=Math.abs(m-e),y=Math.abs(p-t);return[m,p,_,y]}function Uu(e,t){return e>=0?.5*e:t*25*Math.sqrt(-e)}function I0({pos:e,x1:t,y1:s,x2:a,y2:o,c:l}){switch(e){case ze.Left:return[t-Uu(t-a,l),s];case ze.Right:return[t+Uu(a-t,l),s];case ze.Top:return[t,s-Uu(s-o,l)];case ze.Bottom:return[t,s+Uu(o-s,l)]}}function O1({sourceX:e,sourceY:t,sourcePosition:s=ze.Bottom,targetX:a,targetY:o,targetPosition:l=ze.Top,curvature:c=.25}){const[f,m]=I0({pos:s,x1:e,y1:t,x2:a,y2:o,c}),[p,_]=I0({pos:l,x1:a,y1:o,x2:e,y2:t,c}),[y,b,v,S]=A1({sourceX:e,sourceY:t,targetX:a,targetY:o,sourceControlX:f,sourceControlY:m,targetControlX:p,targetControlY:_});return[`M${e},${t} C${f},${m} ${p},${_} ${a},${o}`,y,b,v,S]}function M1({sourceX:e,sourceY:t,targetX:s,targetY:a}){const o=Math.abs(s-e)/2,l=s<e?s+o:s-o,c=Math.abs(a-t)/2,f=a<t?a+c:a-c;return[l,f,o,c]}function y3({sourceNode:e,targetNode:t,selected:s=!1,zIndex:a=0,elevateOnSelect:o=!1,zIndexMode:l="basic"}){if(l==="manual")return a;const c=o&&s?a+1e3:a,f=Math.max(e.parentId||o&&e.selected?e.internals.z:0,t.parentId||o&&t.selected?t.internals.z:0);return c+f}function _3({sourceNode:e,targetNode:t,width:s,height:a,transform:o}){const l=tf(kd(e),kd(t));l.x===l.x2&&(l.x2+=1),l.y===l.y2&&(l.y2+=1);const c={x:-o[0]/o[2],y:-o[1]/o[2],width:s/o[2],height:a/o[2]};return jd(c,nf(l))>0}const b3=({source:e,sourceHandle:t,target:s,targetHandle:a})=>`xy-edge__${e}${t||""}-${s}${a||""}`,v3=(e,t)=>t.some(s=>s.source===e.source&&s.target===e.target&&(s.sourceHandle===e.sourceHandle||!s.sourceHandle&&!e.sourceHandle)&&(s.targetHandle===e.targetHandle||!s.targetHandle&&!e.targetHandle)),x3=(e,t,s={})=>{if(!e.source||!e.target)return s.onError?.("006",Ds.error006()),t;const a=s.getEdgeId||b3;let o;return _1(e)?o={...e}:o={...e,id:a(e)},v3(o,t)?t:(o.sourceHandle===null&&delete o.sourceHandle,o.targetHandle===null&&delete o.targetHandle,t.concat(o))};function z1({sourceX:e,sourceY:t,targetX:s,targetY:a}){const[o,l,c,f]=M1({sourceX:e,sourceY:t,targetX:s,targetY:a});return[`M ${e},${t}L ${s},${a}`,o,l,c,f]}const Z0={[ze.Left]:{x:-1,y:0},[ze.Right]:{x:1,y:0},[ze.Top]:{x:0,y:-1},[ze.Bottom]:{x:0,y:1}},w3=({source:e,sourcePosition:t=ze.Bottom,target:s})=>t===ze.Left||t===ze.Right?e.x<s.x?{x:1,y:0}:{x:-1,y:0}:e.y<s.y?{x:0,y:1}:{x:0,y:-1},P0=(e,t)=>Math.sqrt(Math.pow(t.x-e.x,2)+Math.pow(t.y-e.y,2)),S3=(e,t,s)=>e.x===t.x&&t.x===s.x&&(t.y-e.y)*(s.y-t.y)>=0||e.y===t.y&&t.y===s.y&&(t.x-e.x)*(s.x-t.x)>=0;function k3({source:e,sourcePosition:t=ze.Bottom,target:s,targetPosition:a=ze.Top,center:o,offset:l,stepPosition:c}){const f=Z0[t],m=Z0[a],p={x:e.x+f.x*l,y:e.y+f.y*l},_={x:s.x+m.x*l,y:s.y+m.y*l},y=w3({source:p,sourcePosition:t,target:_}),b=y.x!==0?"x":"y",v=y[b];let S=[],w,k;const j={x:0,y:0},A={x:0,y:0},[,,N,T]=M1({sourceX:e.x,sourceY:e.y,targetX:s.x,targetY:s.y});if(f[b]*m[b]===-1){b==="x"?(w=o.x??p.x+(_.x-p.x)*c,k=o.y??(p.y+_.y)/2):(w=o.x??(p.x+_.x)/2,k=o.y??p.y+(_.y-p.y)*c);const q=[{x:w,y:p.y},{x:w,y:_.y}],Z=[{x:p.x,y:k},{x:_.x,y:k}];f[b]===v?S=b==="x"?q:Z:S=b==="x"?Z:q}else{const q=[{x:p.x,y:_.y}],Z=[{x:_.x,y:p.y}];if(b==="x"?S=f.x===v?Z:q:S=f.y===v?q:Z,t===a){const L=Math.abs(e[b]-s[b]);if(L<=l){const I=Math.min(l-1,l-L);f[b]===v?j[b]=(p[b]>e[b]?-1:1)*I:A[b]=(_[b]>s[b]?-1:1)*I}}if(t!==a){const L=b==="x"?"y":"x",I=f[b]===m[L],D=p[L]>_[L],U=p[L]<_[L];(f[b]===1&&(!I&&D||I&&U)||f[b]!==1&&(!I&&U||I&&D))&&(S=b==="x"?q:Z)}const oe={x:p.x+j.x,y:p.y+j.y},Q={x:_.x+A.x,y:_.y+A.y},ee=Math.max(Math.abs(oe.x-S[0].x),Math.abs(Q.x-S[0].x)),X=Math.max(Math.abs(oe.y-S[0].y),Math.abs(Q.y-S[0].y));ee>=X?(w=(oe.x+Q.x)/2,k=S[0].y):(w=S[0].x,k=(oe.y+Q.y)/2)}const E={x:p.x+j.x,y:p.y+j.y},O={x:_.x+A.x,y:_.y+A.y},R=[];for(const q of[e,E,...S,O,s]){const[Z,oe]=R.slice(-2);oe&&S3(Z,oe,q)&&R.pop(),R.push(q)}return[R,w,k,N,T]}function j3(e,t,s,a){const o=Math.min(P0(e,t)/2,P0(t,s)/2,a),{x:l,y:c}=t;if(e.x===l&&l===s.x||e.y===c&&c===s.y)return`L${l} ${c}`;if(e.y===c){const p=e.x<s.x?-1:1,_=e.y<s.y?1:-1;return`L ${l+o*p},${c}Q ${l},${c} ${l},${c+o*_}`}const f=e.x<s.x?1:-1,m=e.y<s.y?-1:1;return`L ${l},${c+o*m}Q ${l},${c} ${l+o*f},${c}`}function yh({sourceX:e,sourceY:t,sourcePosition:s=ze.Bottom,targetX:a,targetY:o,targetPosition:l=ze.Top,borderRadius:c=5,centerX:f,centerY:m,offset:p=20,stepPosition:_=.5}){const[y,b,v,S,w]=k3({source:{x:e,y:t},sourcePosition:s,target:{x:a,y:o},targetPosition:l,center:{x:f,y:m},offset:p,stepPosition:_});let k=`M${y[0].x} ${y[0].y}`;for(let j=1;j<y.length-1;j++)k+=j3(y[j-1],y[j],y[j+1],c);return k+=`L${y[y.length-1].x} ${y[y.length-1].y}`,[k,b,v,S,w]}function Y0(e){return e&&!!(e.internals.handleBounds||e.handles?.length)&&!!(e.measured.width||e.width||e.initialWidth)}function N3(e){const{sourceNode:t,targetNode:s}=e;if(!Y0(t)||!Y0(s))return null;const a=t.internals.handleBounds||V0(t.handles),o=s.internals.handleBounds||V0(s.handles),l=K0(a?.source??[],e.sourceHandle),c=K0(e.connectionMode===jr.Strict?o?.target??[]:(o?.target??[]).concat(o?.source??[]),e.targetHandle);if(!l||!c)return e.onError?.("008",Ds.error008(l?"target":"source",{id:e.id,sourceHandle:e.sourceHandle,targetHandle:e.targetHandle})),null;const f=l?.position||ze.Bottom,m=c?.position||ze.Top,p=mo(t,l,f),_=mo(s,c,m);return{sourceX:p.x,sourceY:p.y,targetX:_.x,targetY:_.y,sourcePosition:f,targetPosition:m}}function V0(e){if(!e)return null;const t=[],s=[];for(const a of e)a.width=a.width??1,a.height=a.height??1,a.type==="source"?t.push(a):a.type==="target"&&s.push(a);return{source:t,target:s}}function mo(e,t,s=ze.Left,a=!1){const o=(t?.x??0)+e.internals.positionAbsolute.x,l=(t?.y??0)+e.internals.positionAbsolute.y,{width:c,height:f}=t??qs(e);if(a)return{x:o+c/2,y:l+f/2};switch(t?.position??s){case ze.Top:return{x:o+c/2,y:l};case ze.Right:return{x:o+c,y:l+f/2};case ze.Bottom:return{x:o+c/2,y:l+f};case ze.Left:return{x:o,y:l+f/2}}}function K0(e,t){return e&&(t?e.find(s=>s.id===t):e[0])||null}function _h(e,t){return e?typeof e=="string"?e:`${t?`${t}__`:""}${Object.keys(e).sort().map(a=>`${a}=${e[a]}`).join("&")}`:""}function E3(e,{id:t,defaultColor:s,defaultMarkerStart:a,defaultMarkerEnd:o}){const l=new Set;return e.reduce((c,f)=>([f.markerStart||a,f.markerEnd||o].forEach(m=>{if(m&&typeof m=="object"){const p=_h(m,t);l.has(p)||(c.push({id:p,color:m.color||s,...m}),l.add(p))}}),c),[]).sort((c,f)=>c.id.localeCompare(f.id))}const D1=1e3,T3=10,_g={nodeOrigin:[0,0],nodeExtent:Kl,elevateNodesOnSelect:!0,zIndexMode:"basic",defaults:{}},C3={..._g,checkEquality:!0};function bg(e,t){const s={...e};for(const a in t)t[a]!==void 0&&(s[a]=t[a]);return s}function A3(e,t,s){const a=bg(_g,s);for(const o of e.values())if(o.parentId)xg(o,e,t,a);else{const l=cc(o,a.nodeOrigin),c=po(o.extent)?o.extent:a.nodeExtent,f=fo(l,c,qs(o));o.internals.positionAbsolute=f}}function O3(e,t){if(!e.handles)return e.measured?t?.internals.handleBounds:void 0;const s=[],a=[];for(const o of e.handles){const l={id:o.id,width:o.width??1,height:o.height??1,nodeId:e.id,x:o.x,y:o.y,position:o.position,type:o.type};o.type==="source"?s.push(l):o.type==="target"&&a.push(l)}return{source:s,target:a}}function vg(e){return e==="manual"}function bh(e,t,s,a={}){const o=bg(C3,a),l={i:0},c=new Map(t),f=o?.elevateNodesOnSelect&&!vg(o.zIndexMode)?D1:0;let m=e.length>0,p=!1;t.clear(),s.clear();for(const _ of e){let y=c.get(_.id);if(o.checkEquality&&_===y?.internals.userNode)t.set(_.id,y);else{const b=cc(_,o.nodeOrigin),v=po(_.extent)?_.extent:o.nodeExtent,S=fo(b,v,qs(_));y={...o.defaults,..._,measured:{width:_.measured?.width,height:_.measured?.height},internals:{positionAbsolute:S,handleBounds:O3(_,y),z:$1(_,f,o.zIndexMode),userNode:_}},t.set(_.id,y)}(y.measured===void 0||y.measured.width===void 0||y.measured.height===void 0)&&!y.hidden&&(m=!1),_.parentId&&xg(y,t,s,a,l),p||=_.selected??!1}return{nodesInitialized:m,hasSelectedNodes:p}}function M3(e,t){if(!e.parentId)return;const s=t.get(e.parentId);s?s.set(e.id,e):t.set(e.parentId,new Map([[e.id,e]]))}function xg(e,t,s,a,o){const{elevateNodesOnSelect:l,nodeOrigin:c,nodeExtent:f,zIndexMode:m}=bg(_g,a),p=e.parentId,_=t.get(p);if(!_){console.warn(`Parent node ${p} not found. Please make sure that parent nodes are in front of their child nodes in the nodes array.`);return}M3(e,s),o&&!_.parentId&&_.internals.rootParentIndex===void 0&&m==="auto"&&(_.internals.rootParentIndex=++o.i,_.internals.z=_.internals.z+o.i*T3),o&&_.internals.rootParentIndex!==void 0&&(o.i=_.internals.rootParentIndex);const y=l&&!vg(m)?D1:0,{x:b,y:v,z:S}=z3(e,_,c,f,y,m),{positionAbsolute:w}=e.internals,k=b!==w.x||v!==w.y;(k||S!==e.internals.z)&&t.set(e.id,{...e,internals:{...e.internals,positionAbsolute:k?{x:b,y:v}:w,z:S}})}function $1(e,t,s){const a=Ms(e.zIndex)?e.zIndex:0;return vg(s)?a:a+(e.selected?t:0)}function z3(e,t,s,a,o,l){const{x:c,y:f}=t.internals.positionAbsolute,m=qs(e),p=cc(e,s),_=po(e.extent)?fo(p,e.extent,m):p;let y=fo({x:c+_.x,y:f+_.y},a,m);e.extent==="parent"&&(y=v1(y,m,t));const b=$1(e,o,l),v=t.internals.z??0;return{x:y.x,y:y.y,z:v>=b?v+1:b}}function wg(e,t,s,a=[0,0]){const o=[],l=new Map;for(const c of e){const f=t.get(c.parentId);if(!f)continue;const m=l.get(c.parentId)?.expandedRect??Fl(f),p=x1(m,c.rect);l.set(c.parentId,{expandedRect:p,parent:f})}return l.size>0&&l.forEach(({expandedRect:c,parent:f},m)=>{const p=f.internals.positionAbsolute,_=qs(f),y=f.origin??a,b=c.x<p.x?Math.round(Math.abs(p.x-c.x)):0,v=c.y<p.y?Math.round(Math.abs(p.y-c.y)):0,S=Math.max(_.width,Math.round(c.width)),w=Math.max(_.height,Math.round(c.height)),k=(S-_.width)*y[0],j=(w-_.height)*y[1];(b>0||v>0||k||j)&&(o.push({id:m,type:"position",position:{x:f.position.x-b+k,y:f.position.y-v+j}}),s.get(m)?.forEach(A=>{e.some(N=>N.id===A.id)||o.push({id:A.id,type:"position",position:{x:A.position.x+b,y:A.position.y+v}})})),(_.width<c.width||_.height<c.height||b||v)&&o.push({id:m,type:"dimensions",setAttributes:!0,dimensions:{width:S+(b?y[0]*b-k:0),height:w+(v?y[1]*v-j:0)}})}),o}function D3(e,t,s,a,o,l,c){const f=a?.querySelector(".xyflow__viewport");let m=!1;if(!f)return{changes:[],updatedInternals:m};const p=[],_=window.getComputedStyle(f),{m22:y}=new window.DOMMatrixReadOnly(_.transform),b=[];for(const v of e.values()){const S=t.get(v.id);if(!S)continue;if(S.hidden){t.set(S.id,{...S,internals:{...S.internals,handleBounds:void 0}}),m=!0;continue}const w=yg(v.nodeElement),k=S.measured.width!==w.width||S.measured.height!==w.height;if(!!(w.width&&w.height&&(k||!S.internals.handleBounds||v.force))){const A=v.nodeElement.getBoundingClientRect(),N=po(S.extent)?S.extent:l;let{positionAbsolute:T}=S.internals;if(S.parentId&&S.extent==="parent"){const O=t.get(S.parentId);O&&(T=v1(T,w,O))}else N&&(T=fo(T,N,w));const E={...S,measured:w,internals:{...S.internals,positionAbsolute:T,handleBounds:{source:H0("source",v.nodeElement,A,y,S.id),target:H0("target",v.nodeElement,A,y,S.id)}}};t.set(S.id,E),S.parentId&&xg(E,t,s,{nodeOrigin:o,zIndexMode:c}),m=!0,k&&(p.push({id:S.id,type:"dimensions",dimensions:w}),S.expandParent&&S.parentId&&b.push({id:S.id,parentId:S.parentId,rect:Fl(E,o)}))}}if(b.length>0){const v=wg(b,t,s,o);p.push(...v)}return{changes:p,updatedInternals:m}}async function $3({delta:e,panZoom:t,transform:s,translateExtent:a,width:o,height:l}){if(!t||!e.x&&!e.y)return!1;const c=await t.setViewportConstrained({x:s[0]+e.x,y:s[1]+e.y,zoom:s[2]},[[0,0],[o,l]],a);return!!c&&(c.x!==s[0]||c.y!==s[1]||c.k!==s[2])}function G0(e,t,s,a,o,l){let c=o;const f=a.get(c)||new Map;a.set(c,f.set(s,t)),c=`${o}-${e}`;const m=a.get(c)||new Map;if(a.set(c,m.set(s,t)),l){c=`${o}-${e}-${l}`;const p=a.get(c)||new Map;a.set(c,p.set(s,t))}}function R1(e,t,s){e.clear(),t.clear();for(const a of s){const{source:o,target:l,sourceHandle:c=null,targetHandle:f=null}=a,m={edgeId:a.id,source:o,target:l,sourceHandle:c,targetHandle:f},p=`${o}-${c}--${l}-${f}`,_=`${l}-${f}--${o}-${c}`;G0("source",m,_,e,o,c),G0("target",m,p,e,l,f),t.set(a.id,a)}}function L1(e,t){if(!e.parentId)return!1;const s=t.get(e.parentId);return s?s.selected?!0:L1(s,t):!1}function X0(e,t,s){let a=e;do{if(a?.matches?.(t))return!0;if(a===s)return!1;a=a?.parentElement}while(a);return!1}function R3(e,t,s,a,o){const l=new Map;for(const[c,f]of e)if((o?f.id===a:f.selected||f.id===a)&&(o||!f.parentId||!L1(f,e))&&(f.draggable||t&&typeof f.draggable>"u")){const p=e.get(c);p&&l.set(c,{id:c,position:p.position||{x:0,y:0},distance:{x:s.x-p.internals.positionAbsolute.x,y:s.y-p.internals.positionAbsolute.y},extent:p.extent,parentId:p.parentId,origin:p.origin,expandParent:p.expandParent,internals:{positionAbsolute:p.internals.positionAbsolute||{x:0,y:0}},measured:{width:p.measured.width??0,height:p.measured.height??0}})}return l}function Lm({nodeId:e,dragItems:t,nodeLookup:s,dragging:a=!0}){const o=[];for(const[c,f]of t){const m=s.get(c)?.internals.userNode;m&&o.push({...m,position:f.position,dragging:a})}if(!e)return[o[0],o];const l=s.get(e)?.internals.userNode;return[l?{...l,position:t.get(e)?.position||l.position,dragging:a}:o[0],o]}function L3({dragItems:e,snapGrid:t,x:s,y:a}){const o=e.values().next().value;if(!o)return null;const l={x:s-o.distance.x,y:a-o.distance.y},c=dc(l,t);return{x:c.x-l.x,y:c.y-l.y}}function q3({onNodeMouseDown:e,getStoreItems:t,onDragStart:s,onDrag:a,onDragStop:o}){let l={x:null,y:null},c=0,f=new Map,m=!1,p={x:0,y:0},_=null,y=!1,b=null,v=!1,S=!1,w=null;function k({noDragClassName:A,handleSelector:N,domNode:T,isSelectable:E,nodeId:O,nodeClickDistance:R=0}){b=ss(T);function q({x:ee,y:X}){const{nodeLookup:L,nodeExtent:I,snapGrid:D,snapToGrid:U,nodeOrigin:B,onNodeDrag:V,onSelectionDrag:te,onError:z,updateNodePositions:Y}=t();l={x:ee,y:X};let ce=!1;const de=f.size>1,_e=de&&I?gh(uc(f)):null,F=de&&U?L3({dragItems:f,snapGrid:D,x:ee,y:X}):null;for(const[G,P]of f){if(!L.has(G))continue;let ae={x:ee-P.distance.x,y:X-P.distance.y};U&&(ae=F?{x:Math.round(ae.x+F.x),y:Math.round(ae.y+F.y)}:dc(ae,D));let Se=null;if(de&&I&&!P.extent&&_e){const{positionAbsolute:ve}=P.internals,xe=ve.x-_e.x+I[0][0],ke=ve.x+P.measured.width-_e.x2+I[1][0],me=ve.y-_e.y+I[0][1],Ge=ve.y+P.measured.height-_e.y2+I[1][1];Se=[[xe,me],[ke,Ge]]}const{position:je,positionAbsolute:we}=b1({nodeId:G,nextPosition:ae,nodeLookup:L,nodeExtent:Se||I,nodeOrigin:B,onError:z});ce=ce||P.position.x!==je.x||P.position.y!==je.y,P.position=je,P.internals.positionAbsolute=we}if(S=S||ce,!!ce&&(Y(f,!0),w&&(a||V||!O&&te))){const[G,P]=Lm({nodeId:O,dragItems:f,nodeLookup:L});a?.(w,f,G,P),V?.(w,G,P),O||te?.(w,P)}}async function Z(){if(!_)return;const{transform:ee,panBy:X,autoPanSpeed:L,autoPanOnNodeDrag:I}=t();if(!I){m=!1,cancelAnimationFrame(c);return}const[D,U]=hg(p,_,L);(D!==0||U!==0)&&(l.x=(l.x??0)-D/ee[2],l.y=(l.y??0)-U/ee[2],await X({x:D,y:U})&&q(l)),c=requestAnimationFrame(Z)}function oe(ee){const{nodeLookup:X,multiSelectionActive:L,nodesDraggable:I,transform:D,snapGrid:U,snapToGrid:B,selectNodesOnDrag:V,onNodeDragStart:te,onSelectionDragStart:z,unselectNodesAndEdges:Y}=t();y=!0;const ce=!!O&&!L&&!X.get(O)?.selected;(!V||!E)&&!L&&O&&(X.get(O)?.selected||Y()),E&&V&&O&&e?.(O);const de=Rl(ee.sourceEvent,{transform:D,snapGrid:U,snapToGrid:B,containerBounds:_});if(l=de,f=R3(X,I,de,O,ce),f.size>0&&(s||te||!O&&z)){const[_e,F]=Lm({nodeId:O,dragItems:f,nodeLookup:X});s?.(ee.sourceEvent,f,_e,F),te?.(ee.sourceEvent,_e,F),O||z?.(ee.sourceEvent,F)}}const Q=ZS().clickDistance(R).on("start",ee=>{const{domNode:X,nodeDragThreshold:L,transform:I,snapGrid:D,snapToGrid:U}=t();_=X?.getBoundingClientRect()||null,v=!1,S=!1,w=ee.sourceEvent,L===0&&oe(ee),l=Rl(ee.sourceEvent,{transform:I,snapGrid:D,snapToGrid:U,containerBounds:_}),p=zs(ee.sourceEvent,_)}).on("drag",ee=>{const{autoPanOnNodeDrag:X,transform:L,snapGrid:I,snapToGrid:D,nodeDragThreshold:U,nodeLookup:B}=t(),V=Rl(ee.sourceEvent,{transform:L,snapGrid:I,snapToGrid:D,containerBounds:_});if(w=ee.sourceEvent,(ee.sourceEvent.type==="touchmove"&&ee.sourceEvent.touches.length>1||O&&!B.has(O))&&(v=!0),!v){if(!m&&X&&y&&(m=!0,Z()),!y){const te=zs(ee.sourceEvent,_),z=te.x-p.x,Y=te.y-p.y;Math.sqrt(z*z+Y*Y)>U&&oe(ee)}(l.x!==V.xSnapped||l.y!==V.ySnapped)&&f&&y&&(p=zs(ee.sourceEvent,_),q(V))}}).on("end",ee=>{if(!y||v){v&&f.size>0&&t().updateNodePositions(f,!1);return}if(m=!1,y=!1,cancelAnimationFrame(c),f.size>0){const{nodeLookup:X,updateNodePositions:L,onNodeDragStop:I,onSelectionDragStop:D}=t();if(S&&(L(f,!1),S=!1),o||I||!O&&D){const[U,B]=Lm({nodeId:O,dragItems:f,nodeLookup:X,dragging:!1});o?.(ee.sourceEvent,f,U,B),I?.(ee.sourceEvent,U,B),O||D?.(ee.sourceEvent,B)}}}).filter(ee=>{const X=ee.target;return!ee.button&&(!A||!X0(X,`.${A}`,T))&&(!N||X0(X,N,T))});b.call(Q)}function j(){b?.on(".drag",null)}return{update:k,destroy:j}}function U3(e,t,s){const a=[],o={x:e.x-s,y:e.y-s,width:s*2,height:s*2};for(const l of t.values())jd(o,Fl(l))>0&&a.push(l);return a}const B3=250;function H3(e,t,s,a){let o=[],l=1/0;const c=U3(e,s,t+B3);for(const f of c){const m=[...f.internals.handleBounds?.source??[],...f.internals.handleBounds?.target??[]];for(const p of m){if(a.nodeId===p.nodeId&&a.type===p.type&&a.id===p.id)continue;const{x:_,y}=mo(f,p,p.position,!0),b=Math.sqrt(Math.pow(_-e.x,2)+Math.pow(y-e.y,2));b>t||(b<l?(o=[{...p,x:_,y}],l=b):b===l&&o.push({...p,x:_,y}))}}if(!o.length)return null;if(o.length>1){const f=a.type==="source"?"target":"source";return o.find(m=>m.type===f)??o[0]}return o[0]}function q1(e,t,s,a,o,l=!1){const c=a.get(e);if(!c)return null;const f=o==="strict"?c.internals.handleBounds?.[t]:[...c.internals.handleBounds?.source??[],...c.internals.handleBounds?.target??[]],m=(s?f?.find(p=>p.id===s):f?.[0])??null;return m&&l?{...m,...mo(c,m,m.position,!0)}:m}function U1(e,t){return e||(t?.classList.contains("target")?"target":t?.classList.contains("source")?"source":null)}function I3(e,t){let s=null;return t?s=!0:e&&!t&&(s=!1),s}const B1=()=>!0;function Z3(e,{connectionMode:t,connectionRadius:s,handleId:a,nodeId:o,edgeUpdaterType:l,isTarget:c,domNode:f,nodeLookup:m,lib:p,autoPanOnConnect:_,flowId:y,panBy:b,cancelConnection:v,onConnectStart:S,onConnect:w,onConnectEnd:k,isValidConnection:j=B1,onReconnectEnd:A,updateConnection:N,getTransform:T,getFromHandle:E,autoPanSpeed:O,dragThreshold:R=1,handleDomNode:q}){const Z=E1(e.target);let oe=0,Q;const{x:ee,y:X}=zs(e),L=U1(l,q),I=f?.getBoundingClientRect();let D=!1;if(!I||!L)return;const U=q1(o,L,a,m,t);if(!U)return;let B=zs(e,I),V=!1,te=null,z=!1,Y=null;function ce(){if(!_||!I)return;const[je,we]=hg(B,I,O);b({x:je,y:we}),oe=requestAnimationFrame(ce)}const de={...U,nodeId:o,type:L,position:U.position},_e=m.get(o);let G={inProgress:!0,isValid:null,from:mo(_e,de,ze.Left,!0),fromHandle:de,fromPosition:de.position,fromNode:_e,to:B,toHandle:null,toPosition:L0[de.position],toNode:null,pointer:B};function P(){D=!0,N(G),S?.(e,{nodeId:o,handleId:a,handleType:L})}R===0&&P();function ae(je){if(!D){const{x:Ge,y:Ke}=zs(je),rt=Ge-ee,nt=Ke-X;if(!(rt*rt+nt*nt>R*R))return;P()}if(!E()||!de){Se(je);return}const we=T();B=zs(je,I),Q=H3(fc(B,we,!1,[1,1]),s,m,de),V||(ce(),V=!0);const ve=H1(je,{handle:Q,connectionMode:t,fromNodeId:o,fromHandleId:a,fromType:c?"target":"source",isValidConnection:j,doc:Z,lib:p,flowId:y,nodeLookup:m});Y=ve.handleDomNode,te=ve.connection,z=I3(!!Q,ve.isValid);const xe=m.get(o),ke=xe?mo(xe,de,ze.Left,!0):G.from,me={...G,from:ke,isValid:z,to:ve.toHandle&&z?Er({x:ve.toHandle.x,y:ve.toHandle.y},we):B,toHandle:ve.toHandle,toPosition:z&&ve.toHandle?ve.toHandle.position:L0[de.position],toNode:ve.toHandle?m.get(ve.toHandle.nodeId):null,pointer:B};N(me),G=me}function Se(je){if(!("touches"in je&&je.touches.length>0)){if(D){(Q||Y)&&te&&z&&w?.(te);const{inProgress:we,...ve}=G,xe={...ve,toPosition:G.toHandle?G.toPosition:null};k?.(je,xe),l&&A?.(je,xe)}v(),cancelAnimationFrame(oe),V=!1,z=!1,te=null,Y=null,Z.removeEventListener("mousemove",ae),Z.removeEventListener("mouseup",Se),Z.removeEventListener("touchmove",ae),Z.removeEventListener("touchend",Se)}}Z.addEventListener("mousemove",ae),Z.addEventListener("mouseup",Se),Z.addEventListener("touchmove",ae),Z.addEventListener("touchend",Se)}function H1(e,{handle:t,connectionMode:s,fromNodeId:a,fromHandleId:o,fromType:l,doc:c,lib:f,flowId:m,isValidConnection:p=B1,nodeLookup:_}){const y=l==="target",b=t?c.querySelector(`.${f}-flow__handle[data-id="${m}-${t?.nodeId}-${t?.id}-${t?.type}"]`):null,{x:v,y:S}=zs(e),w=c.elementFromPoint(v,S),k=w?.classList.contains(`${f}-flow__handle`)?w:b,j={handleDomNode:k,isValid:!1,connection:null,toHandle:null};if(k){const A=U1(void 0,k),N=k.getAttribute("data-nodeid"),T=k.getAttribute("data-handleid"),E=k.classList.contains("connectable"),O=k.classList.contains("connectableend");if(!N||!A)return j;const R={source:y?N:a,sourceHandle:y?T:o,target:y?a:N,targetHandle:y?o:T};j.connection=R;const Z=E&&O&&(s===jr.Strict?y&&A==="source"||!y&&A==="target":N!==a||T!==o);j.isValid=Z&&p(R),j.toHandle=q1(N,A,T,_,s,!0)}return j}const vh={onPointerDown:Z3,isValid:H1};function P3({domNode:e,panZoom:t,getTransform:s,getViewScale:a}){const o=ss(e);function l({translateExtent:f,width:m,height:p,zoomStep:_=1,pannable:y=!0,zoomable:b=!0,inversePan:v=!1}){const S=N=>{if(N.sourceEvent.type!=="wheel"||!t)return;const T=s(),E=N.sourceEvent.ctrlKey&&Jl()?10:1,O=-N.sourceEvent.deltaY*(N.sourceEvent.deltaMode===1?.05:N.sourceEvent.deltaMode?1:.002)*_,R=T[2]*Math.pow(2,O*E);t.scaleTo(R)};let w=[0,0];const k=N=>{(N.sourceEvent.type==="mousedown"||N.sourceEvent.type==="touchstart")&&(w=[N.sourceEvent.clientX??N.sourceEvent.touches[0].clientX,N.sourceEvent.clientY??N.sourceEvent.touches[0].clientY])},j=N=>{const T=s();if(N.sourceEvent.type!=="mousemove"&&N.sourceEvent.type!=="touchmove"||!t)return;const E=[N.sourceEvent.clientX??N.sourceEvent.touches[0].clientX,N.sourceEvent.clientY??N.sourceEvent.touches[0].clientY],O=[E[0]-w[0],E[1]-w[1]];w=E;const R=a()*Math.max(T[2],Math.log(T[2]))*(v?-1:1),q={x:T[0]-O[0]*R,y:T[1]-O[1]*R},Z=[[0,0],[m,p]];t.setViewportConstrained({x:q.x,y:q.y,zoom:T[2]},Z,f)},A=m1().on("start",k).on("zoom",y?j:null).on("zoom.wheel",b?S:null);o.call(A,{})}function c(){o.on("zoom",null)}return{update:l,destroy:c,pointer:Cs}}const sf=e=>({x:e.x,y:e.y,zoom:e.k}),qm=({x:e,y:t,zoom:s})=>ef.translate(e,t).scale(s),fa=(e,t)=>e.target.closest(`.${t}`),I1=(e,t)=>t===2&&Array.isArray(e)&&e.includes(2),Y3=e=>((e*=2)<=1?e*e*e:(e-=2)*e*e+2)/2,Um=(e,t=0,s=Y3,a=()=>{})=>{const o=typeof t=="number"&&t>0;return o||a(),o?e.transition().duration(t).ease(s).on("end",a):e},Z1=e=>{const t=e.ctrlKey&&Jl()?10:1;return-e.deltaY*(e.deltaMode===1?.05:e.deltaMode?1:.002)*t};function V3({zoomPanValues:e,noWheelClassName:t,d3Selection:s,d3Zoom:a,panOnScrollMode:o,panOnScrollSpeed:l,zoomOnPinch:c,onPanZoomStart:f,onPanZoom:m,onPanZoomEnd:p}){return _=>{if(fa(_,t))return _.ctrlKey&&_.preventDefault(),!1;_.preventDefault(),_.stopImmediatePropagation();const y=s.property("__zoom").k||1;if(_.ctrlKey&&c){const k=Cs(_),j=Z1(_),A=y*Math.pow(2,j);a.scaleTo(s,A,k,_);return}const b=_.deltaMode===1?20:1;let v=o===so.Vertical?0:_.deltaX*b,S=o===so.Horizontal?0:_.deltaY*b;!Jl()&&_.shiftKey&&o!==so.Vertical&&(v=_.deltaY*b,S=0),a.translateBy(s,-(v/y)*l,-(S/y)*l,{internal:!0});const w=sf(s.property("__zoom"));clearTimeout(e.panScrollTimeout),e.isPanScrolling?m?.(_,w):(e.isPanScrolling=!0,f?.(_,w)),e.panScrollTimeout=setTimeout(()=>{p?.(_,w),e.isPanScrolling=!1},150)}}function K3({noWheelClassName:e,preventScrolling:t,d3ZoomHandler:s}){return function(a,o){const l=a.type==="wheel",c=!t&&l&&!a.ctrlKey,f=fa(a,e);if(a.ctrlKey&&l&&f&&a.preventDefault(),c||f)return null;a.preventDefault(),s.call(this,a,o)}}function G3({zoomPanValues:e,onDraggingChange:t,onPanZoomStart:s}){return a=>{if(a.sourceEvent?.internal)return;const o=sf(a.transform);e.mouseButton=a.sourceEvent?.button||0,e.isZoomingOrPanning=!0,e.prevViewport=o,a.sourceEvent?.type==="mousedown"&&t(!0),s&&s?.(a.sourceEvent,o)}}function X3({zoomPanValues:e,panOnDrag:t,onPaneContextMenu:s,onTransformChange:a,onPanZoom:o}){return l=>{e.usedRightMouseButton=!!(s&&I1(t,e.mouseButton??0)),l.sourceEvent?.sync||a([l.transform.x,l.transform.y,l.transform.k]),o&&!l.sourceEvent?.internal&&o?.(l.sourceEvent,sf(l.transform))}}function F3({zoomPanValues:e,panOnDrag:t,panOnScroll:s,onDraggingChange:a,onPanZoomEnd:o,onPaneContextMenu:l}){return c=>{if(!c.sourceEvent?.internal&&(e.isZoomingOrPanning=!1,l&&I1(t,e.mouseButton??0)&&!e.usedRightMouseButton&&c.sourceEvent&&l(c.sourceEvent),e.usedRightMouseButton=!1,a(!1),o)){const f=sf(c.transform);e.prevViewport=f,clearTimeout(e.timerId),e.timerId=setTimeout(()=>{o?.(c.sourceEvent,f)},s?150:0)}}}function J3({panActivationKeyPressed:e,zoomActivationKeyPressed:t,zoomOnScroll:s,zoomOnPinch:a,panOnDrag:o,panOnScroll:l,zoomOnDoubleClick:c,userSelectionActive:f,noWheelClassName:m,noPanClassName:p,lib:_,connectionInProgress:y}){return b=>{const v=t||s,S=a&&b.ctrlKey,w=b.type==="wheel";if(b.button===1&&b.type==="mousedown"&&(fa(b,`${_}-flow__node`)||fa(b,`${_}-flow__edge`)||fa(b,`${_}-flow__selection`)||fa(b,`${_}-flow__nodesselection`)))return!0;if(!o&&!v&&!l&&!c&&!a||f||y&&!w||fa(b,m)&&w||fa(b,p)&&(!w||l&&w&&!t)||!a&&b.ctrlKey&&w)return!1;if(!a&&b.type==="touchstart"&&b.touches?.length>1)return b.preventDefault(),!1;if(!v&&!l&&!S&&w||!o&&(b.type==="mousedown"||b.type==="touchstart")||Array.isArray(o)&&!o.includes(b.button)&&b.type==="mousedown")return!1;const k=Array.isArray(o)&&o.includes(b.button)||!b.button||b.button<=1;return(!b.ctrlKey||w||e)&&k}}function Q3({domNode:e,minZoom:t,maxZoom:s,translateExtent:a,viewport:o,onPanZoom:l,onPanZoomStart:c,onPanZoomEnd:f,onDraggingChange:m}){const p={isZoomingOrPanning:!1,usedRightMouseButton:!1,prevViewport:{},mouseButton:0,timerId:void 0,panScrollTimeout:void 0,isPanScrolling:!1},_=e.getBoundingClientRect();let y=[[0,0],[_.width,_.height]];const b=typeof ResizeObserver<"u"?new ResizeObserver(X=>{const L=X[0];L?.contentRect&&(y=[[0,0],[L.contentRect.width,L.contentRect.height]])}):null;b?.observe(e);const v=m1().extent(()=>y).scaleExtent([t,s]).translateExtent(a),S=ss(e).call(v);T({x:o.x,y:o.y,zoom:Nr(o.zoom,t,s)},[[0,0],[_.width,_.height]],a);const w=S.on("wheel.zoom"),k=S.on("dblclick.zoom");v.wheelDelta(Z1);async function j(X,L){return S?new Promise(I=>{v?.interpolate(L?.interpolate==="linear"?$l:Xu).transform(Um(S,L?.duration,L?.ease,()=>I(!0)),X)}):!1}function A({noWheelClassName:X,noPanClassName:L,onPaneContextMenu:I,userSelectionActive:D,panOnScroll:U,panOnDrag:B,panOnScrollMode:V,panOnScrollSpeed:te,preventScrolling:z,zoomOnPinch:Y,zoomOnScroll:ce,zoomOnDoubleClick:de,panActivationKeyPressed:_e=!1,zoomActivationKeyPressed:F,lib:G,onTransformChange:P,connectionInProgress:ae,paneClickDistance:Se,selectionOnDrag:je}){D&&!p.isZoomingOrPanning&&v.on("zoom",null);const we=U&&!F&&!D;v.clickDistance(je?1/0:!Ms(Se)||Se<0?0:Se);const ve=we?V3({zoomPanValues:p,noWheelClassName:X,d3Selection:S,d3Zoom:v,panOnScrollMode:V,panOnScrollSpeed:te,zoomOnPinch:Y,onPanZoomStart:c,onPanZoom:l,onPanZoomEnd:f}):K3({noWheelClassName:X,preventScrolling:z,d3ZoomHandler:w});S.on("wheel.zoom",ve,{passive:!1});const xe=G3({zoomPanValues:p,onDraggingChange:m,onPanZoomStart:c});v.on("start",xe);const ke=X3({zoomPanValues:p,panOnDrag:B,onPaneContextMenu:!!I,onPanZoom:l,onTransformChange:P});v.on("zoom",ke);const me=F3({zoomPanValues:p,panOnDrag:B,panOnScroll:U,onPaneContextMenu:I,onPanZoomEnd:f,onDraggingChange:m});v.on("end",me);const Ge=J3({panActivationKeyPressed:_e,zoomActivationKeyPressed:F,panOnDrag:B,zoomOnScroll:ce,panOnScroll:U,zoomOnDoubleClick:de,zoomOnPinch:Y,userSelectionActive:D,noPanClassName:L,noWheelClassName:X,lib:G,connectionInProgress:ae});v.filter(Ge),de?S.on("dblclick.zoom",k):S.on("dblclick.zoom",null)}function N(){v.on("zoom",null),b?.disconnect()}async function T(X,L,I){const D=qm(X),U=v?.constrain()(D,L,I);return U&&await j(U),U}async function E(X,L){const I=qm(X);return await j(I,L),I}function O(X){if(S){const L=qm(X),I=S.property("__zoom");(I.k!==X.zoom||I.x!==X.x||I.y!==X.y)&&v?.transform(S,L,null,{sync:!0})}}function R(){const X=S?p1(S.node()):{x:0,y:0,k:1};return{x:X.x,y:X.y,zoom:X.k}}async function q(X,L){return S?new Promise(I=>{v?.interpolate(L?.interpolate==="linear"?$l:Xu).scaleTo(Um(S,L?.duration,L?.ease,()=>I(!0)),X)}):!1}async function Z(X,L){return S?new Promise(I=>{v?.interpolate(L?.interpolate==="linear"?$l:Xu).scaleBy(Um(S,L?.duration,L?.ease,()=>I(!0)),X)}):!1}function oe(X){v?.scaleExtent(X)}function Q(X){v?.translateExtent(X)}function ee(X){const L=!Ms(X)||X<0?0:X;v?.clickDistance(L)}return{update:A,destroy:N,setViewport:E,setViewportConstrained:T,getViewport:R,scaleTo:q,scaleBy:Z,setScaleExtent:oe,setTranslateExtent:Q,syncViewport:O,setClickDistance:ee}}var Tr;(function(e){e.Line="line",e.Handle="handle"})(Tr||(Tr={}));function W3({width:e,prevWidth:t,height:s,prevHeight:a,affectsX:o,affectsY:l}){const c=e-t,f=s-a,m=[c>0?1:c<0?-1:0,f>0?1:f<0?-1:0];return c&&o&&(m[0]=m[0]*-1),f&&l&&(m[1]=m[1]*-1),m}function F0(e){const t=e.includes("right")||e.includes("left"),s=e.includes("bottom")||e.includes("top"),a=e.includes("left"),o=e.includes("top");return{isHorizontal:t,isVertical:s,affectsX:a,affectsY:o}}function ca(e,t){return Math.max(0,t-e)}function ua(e,t){return Math.max(0,e-t)}function Bu(e,t,s){return Math.max(0,t-e,e-s)}function J0(e,t){return e?!t:t}function e4(e,t,s,a,o,l,c,f){let{affectsX:m,affectsY:p}=t;const{isHorizontal:_,isVertical:y}=t,b=_&&y,{xSnapped:v,ySnapped:S}=s,{minWidth:w,maxWidth:k,minHeight:j,maxHeight:A}=a,{x:N,y:T,width:E,height:O,aspectRatio:R}=e;let q=Math.floor(_?v-e.pointerX:0),Z=Math.floor(y?S-e.pointerY:0);const oe=E+(m?-q:q),Q=O+(p?-Z:Z),ee=-l[0]*E,X=-l[1]*O;let L=Bu(oe,w,k),I=Bu(Q,j,A);if(c){let B=0,V=0;m&&q<0?B=ca(N+q+ee,c[0][0]):!m&&q>0&&(B=ua(N+oe+ee,c[1][0])),p&&Z<0?V=ca(T+Z+X,c[0][1]):!p&&Z>0&&(V=ua(T+Q+X,c[1][1])),L=Math.max(L,B),I=Math.max(I,V)}if(f){let B=0,V=0;m&&q>0?B=ua(N+q,f[0][0]):!m&&q<0&&(B=ca(N+oe,f[1][0])),p&&Z>0?V=ua(T+Z,f[0][1]):!p&&Z<0&&(V=ca(T+Q,f[1][1])),L=Math.max(L,B),I=Math.max(I,V)}if(o){if(_){const B=Bu(oe/R,j,A)*R;if(L=Math.max(L,B),c){let V=0;!m&&!p||m&&!p&&b?V=ua(T+X+oe/R,c[1][1])*R:V=ca(T+X+(m?q:-q)/R,c[0][1])*R,L=Math.max(L,V)}if(f){let V=0;!m&&!p||m&&!p&&b?V=ca(T+oe/R,f[1][1])*R:V=ua(T+(m?q:-q)/R,f[0][1])*R,L=Math.max(L,V)}}if(y){const B=Bu(Q*R,w,k)/R;if(I=Math.max(I,B),c){let V=0;!m&&!p||p&&!m&&b?V=ua(N+Q*R+ee,c[1][0])/R:V=ca(N+(p?Z:-Z)*R+ee,c[0][0])/R,I=Math.max(I,V)}if(f){let V=0;!m&&!p||p&&!m&&b?V=ca(N+Q*R,f[1][0])/R:V=ua(N+(p?Z:-Z)*R,f[0][0])/R,I=Math.max(I,V)}}}Z=Z+(Z<0?I:-I),q=q+(q<0?L:-L),o&&(b?oe>Q*R?Z=(J0(m,p)?-q:q)/R:q=(J0(m,p)?-Z:Z)*R:_?(Z=q/R,p=m):(q=Z*R,m=p));const D=m?N+q:N,U=p?T+Z:T;return{width:E+(m?-q:q),height:O+(p?-Z:Z),x:l[0]*q*(m?-1:1)+D,y:l[1]*Z*(p?-1:1)+U}}const P1={width:0,height:0,x:0,y:0},t4={...P1,pointerX:0,pointerY:0,aspectRatio:1};function n4(e,t,s){const a=t.position.x+e.position.x,o=t.position.y+e.position.y,l=e.measured.width??0,c=e.measured.height??0,f=s[0]*l,m=s[1]*c;return[[a-f,o-m],[a+l-f,o+c-m]]}function s4({domNode:e,nodeId:t,getStoreItems:s,onChange:a,onEnd:o}){const l=ss(e);let c={controlDirection:F0("bottom-right"),boundaries:{minWidth:0,minHeight:0,maxWidth:Number.MAX_VALUE,maxHeight:Number.MAX_VALUE},resizeDirection:void 0,keepAspectRatio:!1};function f({controlPosition:p,boundaries:_,keepAspectRatio:y,resizeDirection:b,onResizeStart:v,onResize:S,onResizeEnd:w,shouldResize:k}){let j={...P1},A={...t4};c={boundaries:_,resizeDirection:b,keepAspectRatio:y,controlDirection:F0(p)};let N,T=null,E=[],O,R,q;const Z=ZS().on("start",oe=>{const{nodeLookup:Q,transform:ee,snapGrid:X,snapToGrid:L,nodeOrigin:I,paneDomNode:D}=s();if(N=Q.get(t),!N)return;T=D?.getBoundingClientRect()??null;const{xSnapped:U,ySnapped:B}=Rl(oe.sourceEvent,{transform:ee,snapGrid:X,snapToGrid:L,containerBounds:T});j={width:N.measured.width??0,height:N.measured.height??0,x:N.position.x??0,y:N.position.y??0},A={...j,pointerX:U,pointerY:B,aspectRatio:j.width/j.height},O=void 0,R=po(N.extent)?N.extent:void 0,N.parentId&&(N.extent==="parent"||N.expandParent)&&(O=Q.get(N.parentId)),O&&N.extent==="parent"&&(R=[[0,0],[O.measured.width,O.measured.height]]),E=[],q=void 0;for(const[V,te]of Q)if(te.parentId===t&&(E.push({id:V,position:{...te.position},extent:te.extent}),te.extent==="parent"||te.expandParent)){const z=n4(te,N,te.origin??I);q?q=[[Math.min(z[0][0],q[0][0]),Math.min(z[0][1],q[0][1])],[Math.max(z[1][0],q[1][0]),Math.max(z[1][1],q[1][1])]]:q=z}v?.(oe,{...j})}).on("drag",oe=>{const{transform:Q,snapGrid:ee,snapToGrid:X,nodeOrigin:L}=s(),I=Rl(oe.sourceEvent,{transform:Q,snapGrid:ee,snapToGrid:X,containerBounds:T}),D=[];if(!N)return;const{x:U,y:B,width:V,height:te}=j,z={},Y=N.origin??L,{width:ce,height:de,x:_e,y:F}=e4(A,c.controlDirection,I,c.boundaries,c.keepAspectRatio,Y,R,q),G=ce!==V,P=de!==te,ae=_e!==U&&G,Se=F!==B&&P;if(!ae&&!Se&&!G&&!P)return;const je={prevValues:{...j},startX:A.x,startY:A.y,childNodes:E.map(ke=>({...ke,position:{...ke.position}}))};if((ae||Se||Y[0]===1||Y[1]===1)&&(z.x=ae?_e:j.x,z.y=Se?F:j.y,j.x=z.x,j.y=z.y,E.length>0)){const ke=_e-U,me=F-B;for(const Ge of E)Ge.position={x:Ge.position.x-ke+Y[0]*(ce-V),y:Ge.position.y-me+Y[1]*(de-te)},D.push(Ge)}if((G||P)&&(z.width=G&&(!c.resizeDirection||c.resizeDirection==="horizontal")?ce:j.width,z.height=P&&(!c.resizeDirection||c.resizeDirection==="vertical")?de:j.height,j.width=z.width,j.height=z.height),O&&N.expandParent){const ke=Y[0]*(z.width??0);z.x&&z.x<ke&&(j.x=ke,A.x=A.x-(z.x-ke));const me=Y[1]*(z.height??0);z.y&&z.y<me&&(j.y=me,A.y=A.y-(z.y-me))}const we=W3({width:j.width,prevWidth:V,height:j.height,prevHeight:te,affectsX:c.controlDirection.affectsX,affectsY:c.controlDirection.affectsY}),ve={...j,direction:we};if(k?.(oe,ve)===!1){j=je.prevValues,A.x=je.startX,A.y=je.startY,E.forEach((ke,me)=>{ke.position=je.childNodes[me].position});return}S?.(oe,ve),a(z,D)}).on("end",oe=>{w?.(oe,{...j}),o?.({...j})});l.call(Z)}function m(){l.on(".drag",null)}return{update:f,destroy:m}}var Bm={exports:{}},Hm={},Im={exports:{}},Zm={};var Q0;function i4(){if(Q0)return Zm;Q0=1;var e=Wl();function t(y,b){return y===b&&(y!==0||1/y===1/b)||y!==y&&b!==b}var s=typeof Object.is=="function"?Object.is:t,a=e.useState,o=e.useEffect,l=e.useLayoutEffect,c=e.useDebugValue;function f(y,b){var v=b(),S=a({inst:{value:v,getSnapshot:b}}),w=S[0].inst,k=S[1];return l(function(){w.value=v,w.getSnapshot=b,m(w)&&k({inst:w})},[y,v,b]),o(function(){return m(w)&&k({inst:w}),y(function(){m(w)&&k({inst:w})})},[y]),c(v),v}function m(y){var b=y.getSnapshot;y=y.value;try{var v=b();return!s(y,v)}catch{return!0}}function p(y,b){return b()}var _=typeof window>"u"||typeof window.document>"u"||typeof window.document.createElement>"u"?p:f;return Zm.useSyncExternalStore=e.useSyncExternalStore!==void 0?e.useSyncExternalStore:_,Zm}var W0;function a4(){return W0||(W0=1,Im.exports=i4()),Im.exports}var ex;function o4(){if(ex)return Hm;ex=1;var e=Wl(),t=a4();function s(p,_){return p===_&&(p!==0||1/p===1/_)||p!==p&&_!==_}var a=typeof Object.is=="function"?Object.is:s,o=t.useSyncExternalStore,l=e.useRef,c=e.useEffect,f=e.useMemo,m=e.useDebugValue;return Hm.useSyncExternalStoreWithSelector=function(p,_,y,b,v){var S=l(null);if(S.current===null){var w={hasValue:!1,value:null};S.current=w}else w=S.current;S=f(function(){function j(O){if(!A){if(A=!0,N=O,O=b(O),v!==void 0&&w.hasValue){var R=w.value;if(v(R,O))return T=R}return T=O}if(R=T,a(N,O))return R;var q=b(O);return v!==void 0&&v(R,q)?(N=O,R):(N=O,T=q)}var A=!1,N,T,E=y===void 0?null:y;return[function(){return j(_())},E===null?void 0:function(){return j(E())}]},[_,y,b,v]);var k=o(p,S[0],S[1]);return c(function(){w.hasValue=!0,w.value=k},[k]),m(k),k},Hm}var tx;function r4(){return tx||(tx=1,Bm.exports=o4()),Bm.exports}var l4=r4();const c4=qx(l4),u4={},nx=e=>{let t;const s=new Set,a=(_,y)=>{const b=typeof _=="function"?_(t):_;if(!Object.is(b,t)){const v=t;t=y??(typeof b!="object"||b===null)?b:Object.assign({},t,b),s.forEach(S=>S(t,v))}},o=()=>t,m={setState:a,getState:o,getInitialState:()=>p,subscribe:_=>(s.add(_),()=>s.delete(_)),destroy:()=>{(u4?"production":void 0)!=="production"&&console.warn("[DEPRECATED] The `destroy` method will be unsupported in a future version. Instead use unsubscribe function returned by subscribe. Everything will be garbage-collected if store is garbage-collected."),s.clear()}},p=t=e(a,o,m);return m},d4=e=>e?nx(e):nx,{useDebugValue:f4}=kh,{useSyncExternalStoreWithSelector:p4}=c4,m4=e=>e;function Y1(e,t=m4,s){const a=p4(e.subscribe,e.getState,e.getServerState||e.getInitialState,t,s);return f4(a),a}const sx=(e,t)=>{const s=d4(e),a=(o,l=t)=>Y1(s,o,l);return Object.assign(a,s),a},h4=(e,t)=>e?sx(e,t):sx;function Lt(e,t){if(Object.is(e,t))return!0;if(typeof e!="object"||e===null||typeof t!="object"||t===null)return!1;if(e instanceof Map&&t instanceof Map){if(e.size!==t.size)return!1;for(const[a,o]of e)if(!Object.is(o,t.get(a)))return!1;return!0}if(e instanceof Set&&t instanceof Set){if(e.size!==t.size)return!1;for(const a of e)if(!t.has(a))return!1;return!0}const s=Object.keys(e);if(s.length!==Object.keys(t).length)return!1;for(const a of s)if(!Object.prototype.hasOwnProperty.call(t,a)||!Object.is(e[a],t[a]))return!1;return!0}Ux();const af=C.createContext(null),g4=af.Provider,V1=Ds.error001("react");function it(e,t){const s=C.useContext(af);if(s===null)throw new Error(V1);return Y1(s,e,t)}function Et(){const e=C.useContext(af);if(e===null)throw new Error(V1);return C.useMemo(()=>({getState:e.getState,setState:e.setState,subscribe:e.subscribe}),[e])}const ix={display:"none"},y4={position:"absolute",width:1,height:1,margin:-1,border:0,padding:0,overflow:"hidden",clip:"rect(0px, 0px, 0px, 0px)",clipPath:"inset(100%)"},K1="react-flow__node-desc",G1="react-flow__edge-desc",_4="react-flow__aria-live",b4=e=>e.ariaLiveMessage,v4=e=>e.ariaLabelConfig;function x4({rfId:e}){const t=it(b4);return d.jsx("div",{id:`${_4}-${e}`,"aria-live":"assertive","aria-atomic":"true",style:y4,children:t})}function w4({rfId:e,disableKeyboardA11y:t}){const s=it(v4);return d.jsxs(d.Fragment,{children:[d.jsx("div",{id:`${K1}-${e}`,style:ix,children:t?s["node.a11yDescription.default"]:s["node.a11yDescription.keyboardDisabled"]}),d.jsx("div",{id:`${G1}-${e}`,style:ix,children:s["edge.a11yDescription.default"]}),!t&&d.jsx(x4,{rfId:e})]})}const Cr=C.forwardRef(({position:e="top-left",children:t,className:s,style:a,...o},l)=>{const c=`${e}`.split("-");return d.jsx("div",{className:an(["react-flow__panel",s,...c]),style:a,ref:l,...o,children:t})});Cr.displayName="Panel";const ax="https://reactflow.dev?utm_source=attribution";function S4({proOptions:e,position:t="bottom-right"}){return C.useEffect(()=>{},[]),e?.hideAttribution?null:d.jsx(Cr,{position:t,className:"react-flow__attribution","data-message":`Please only hide this attribution when you are subscribed to React Flow Pro: ${ax}`,children:d.jsx("a",{href:ax,target:"_blank",rel:"noopener noreferrer","aria-label":"React Flow attribution",children:"React Flow"})})}const k4=e=>{const t=[],s=[];for(const[,a]of e.nodeLookup)a.selected&&t.push(a.internals.userNode);for(const[,a]of e.edgeLookup)a.selected&&s.push(a);return{selectedNodes:t,selectedEdges:s}},Hu=e=>e.id;function j4(e,t){return Lt(e.selectedNodes.map(Hu),t.selectedNodes.map(Hu))&&Lt(e.selectedEdges.map(Hu),t.selectedEdges.map(Hu))}function N4({onSelectionChange:e}){const t=Et(),{selectedNodes:s,selectedEdges:a}=it(k4,j4);return C.useEffect(()=>{const o={nodes:s,edges:a};e?.(o),t.getState().onSelectionChangeHandlers.forEach(l=>l(o))},[s,a,e]),null}const E4=e=>!!e.onSelectionChangeHandlers;function T4({onSelectionChange:e}){const t=it(E4);return e||t?d.jsx(N4,{onSelectionChange:e}):null}const X1=[0,0],C4={x:0,y:0,zoom:1},A4=["nodes","edges","defaultNodes","defaultEdges","onConnect","onConnectStart","onConnectEnd","onClickConnectStart","onClickConnectEnd","nodesDraggable","autoPanOnNodeFocus","nodesConnectable","nodesFocusable","edgesFocusable","edgesReconnectable","elevateNodesOnSelect","elevateEdgesOnSelect","minZoom","maxZoom","nodeExtent","onNodesChange","onEdgesChange","elementsSelectable","connectionMode","snapGrid","snapToGrid","translateExtent","connectOnClick","defaultEdgeOptions","fitView","fitViewOptions","onNodesDelete","onEdgesDelete","onDelete","onNodeDrag","onNodeDragStart","onNodeDragStop","onSelectionDrag","onSelectionDragStart","onSelectionDragStop","onMoveStart","onMove","onMoveEnd","noPanClassName","nodeOrigin","autoPanOnConnect","autoPanOnNodeDrag","onError","connectionRadius","isValidConnection","selectNodesOnDrag","nodeDragThreshold","connectionDragThreshold","onBeforeDelete","debug","autoPanSpeed","ariaLabelConfig","zIndexMode"],ox=[...A4,"rfId"],O4=e=>({setNodes:e.setNodes,setEdges:e.setEdges,setMinZoom:e.setMinZoom,setMaxZoom:e.setMaxZoom,setTranslateExtent:e.setTranslateExtent,setNodeExtent:e.setNodeExtent,reset:e.reset,setDefaultNodesAndEdges:e.setDefaultNodesAndEdges}),rx={translateExtent:Kl,nodeOrigin:X1,minZoom:.5,maxZoom:2,elementsSelectable:!0,noPanClassName:"nopan",rfId:"1"};function M4(e){const{setNodes:t,setEdges:s,setMinZoom:a,setMaxZoom:o,setTranslateExtent:l,setNodeExtent:c,reset:f,setDefaultNodesAndEdges:m}=it(O4,Lt),p=Et();C.useEffect(()=>(m(e.defaultNodes,e.defaultEdges),()=>{_.current=rx,f()}),[]);const _=C.useRef(rx);return C.useEffect(()=>{for(const y of ox){const b=e[y],v=_.current[y];b!==v&&(typeof e[y]>"u"||(y==="nodes"?t(b):y==="edges"?s(b):y==="minZoom"?a(b):y==="maxZoom"?o(b):y==="translateExtent"?l(b):y==="nodeExtent"?c(b):y==="ariaLabelConfig"?p.setState({ariaLabelConfig:h3(b)}):y==="fitView"?p.setState({fitViewQueued:b}):y==="fitViewOptions"?p.setState({fitViewOptions:b}):p.setState({[y]:b})))}_.current=e},ox.map(y=>e[y])),null}function lx(){return typeof window>"u"||!window.matchMedia?null:window.matchMedia("(prefers-color-scheme: dark)")}function z4(e){const[t,s]=C.useState(e==="system"?null:e);return C.useEffect(()=>{if(e!=="system"){s(e);return}const a=lx(),o=()=>s(a?.matches?"dark":"light");return o(),a?.addEventListener("change",o),()=>{a?.removeEventListener("change",o)}},[e]),t!==null?t:lx()?.matches?"dark":"light"}const cx=typeof document<"u"?document:null;function Ql(e=null,t={target:cx,actInsideInputWithModifier:!0}){const[s,a]=C.useState(!1),o=C.useRef(!1),l=C.useRef(new Set([])),[c,f]=C.useMemo(()=>{if(e!==null){const p=(Array.isArray(e)?e:[e]).filter(y=>typeof y=="string").map(y=>y.replace(/\+/g,`
`).replace(`

`,`
+`).split(`
`)),_=p.reduce((y,b)=>y.concat(...b),[]);return[p,_]}return[[],[]]},[e]);return C.useEffect(()=>{const m=t?.target??cx,p=t?.actInsideInputWithModifier??!0;if(e!==null){const _=v=>{if(o.current=v.ctrlKey||v.metaKey||v.shiftKey||v.altKey,(!o.current||o.current&&!p)&&T1(v))return!1;const w=dx(v.code,f);if(l.current.add(v[w]),ux(c,l.current,!1)){const k=v.composedPath?.()?.[0]||v.target,j=k?.nodeName==="BUTTON"||k?.nodeName==="A";t.preventDefault!==!1&&(o.current||!j)&&v.preventDefault(),a(!0)}},y=v=>{const S=dx(v.code,f);ux(c,l.current,!0)?(a(!1),l.current.clear()):l.current.delete(v[S]),v.key==="Meta"&&l.current.clear(),o.current=!1},b=()=>{l.current.clear(),a(!1)};return m?.addEventListener("keydown",_),m?.addEventListener("keyup",y),window.addEventListener("blur",b),window.addEventListener("contextmenu",b),()=>{m?.removeEventListener("keydown",_),m?.removeEventListener("keyup",y),window.removeEventListener("blur",b),window.removeEventListener("contextmenu",b)}}},[e,a]),s}function ux(e,t,s){return e.filter(a=>s||a.length===t.size).some(a=>a.every(o=>t.has(o)))}function dx(e,t){return t.includes(e)?"code":"key"}const D4=()=>{const e=Et();return C.useMemo(()=>({zoomIn:async t=>{const{panZoom:s}=e.getState();return s?s.scaleBy(1.2,t):!1},zoomOut:async t=>{const{panZoom:s}=e.getState();return s?s.scaleBy(1/1.2,t):!1},zoomTo:async(t,s)=>{const{panZoom:a}=e.getState();return a?a.scaleTo(t,s):!1},getZoom:()=>e.getState().transform[2],setViewport:async(t,s)=>{const{transform:[a,o,l],panZoom:c}=e.getState();return c?(await c.setViewport({x:t.x??a,y:t.y??o,zoom:t.zoom??l},s),!0):!1},getViewport:()=>{const[t,s,a]=e.getState().transform;return{x:t,y:s,zoom:a}},setCenter:async(t,s,a)=>e.getState().setCenter(t,s,a),fitBounds:async(t,s)=>{const{width:a,height:o,minZoom:l,maxZoom:c,panZoom:f}=e.getState(),m=gg(t,a,o,l,c,s?.padding??.1);return f?(await f.setViewport(m,{duration:s?.duration,ease:s?.ease,interpolate:s?.interpolate}),!0):!1},screenToFlowPosition:(t,s={})=>{const{transform:a,snapGrid:o,snapToGrid:l,domNode:c}=e.getState();if(!c)return t;const{x:f,y:m}=c.getBoundingClientRect(),p={x:t.x-f,y:t.y-m},_=s.snapGrid??o,y=s.snapToGrid??l;return fc(p,a,y,_)},flowToScreenPosition:t=>{const{transform:s,domNode:a}=e.getState();if(!a)return t;const{x:o,y:l}=a.getBoundingClientRect(),c=Er(t,s);return{x:c.x+o,y:c.y+l}}}),[])};function F1(e,t){const s=[],a=new Map,o=[];for(const l of e)if(l.type==="add"){o.push(l);continue}else if(l.type==="remove"||l.type==="replace")a.set(l.id,[l]);else{const c=a.get(l.id);c?c.push(l):a.set(l.id,[l])}for(const l of t){const c=a.get(l.id);if(!c){s.push(l);continue}if(c[0].type==="remove")continue;if(c[0].type==="replace"){s.push({...c[0].item});continue}const f={...l};for(const m of c)$4(m,f);s.push(f)}return o.length&&o.forEach(l=>{l.index!==void 0?s.splice(l.index,0,{...l.item}):s.push({...l.item})}),s}function $4(e,t){switch(e.type){case"select":{t.selected=e.selected;break}case"position":{typeof e.position<"u"&&(t.position=e.position),typeof e.dragging<"u"&&(t.dragging=e.dragging);break}case"dimensions":{typeof e.dimensions<"u"&&(t.measured={...e.dimensions},e.setAttributes&&((e.setAttributes===!0||e.setAttributes==="width")&&(t.width=e.dimensions.width),(e.setAttributes===!0||e.setAttributes==="height")&&(t.height=e.dimensions.height))),typeof e.resizing=="boolean"&&(t.resizing=e.resizing);break}}}function R4(e,t){return F1(e,t)}function L4(e,t){return F1(e,t)}function Ga(e,t){return{id:e,type:"select",selected:t}}function cr(e,t=new Set){const s=[];for(const[a,o]of e){const l=t.has(a);!(o.selected===void 0&&!l)&&o.selected!==l&&s.push(Ga(o.id,l))}return s}function fx({items:e=[],lookup:t}){const s=[],a=new Map(e.map(o=>[o.id,o]));for(const[o,l]of e.entries()){const c=t.get(l.id),f=c?.internals?.userNode??c;f!==void 0&&f!==l&&s.push({id:l.id,item:l,type:"replace"}),f===void 0&&s.push({item:l,type:"add",index:o})}for(const[o]of t)a.get(o)===void 0&&s.push({id:o,type:"remove"});return s}function px(e){return{id:e.id,type:"remove"}}const q4=S1();function U4(e,t,s={}){return x3(e,t,{...s,onError:s.onError??q4})}const mx=e=>o3(e),B4=e=>_1(e);function J1(e){return C.forwardRef(e)}const Q1=typeof window<"u"?C.useLayoutEffect:C.useEffect;function hx(e){const[t,s]=C.useState(BigInt(0)),[a]=C.useState(()=>H4(()=>s(o=>o+BigInt(1))));return Q1(()=>{const o=a.get();o.length&&(e(o),a.reset())},[t]),a}function H4(e){let t=[];return{get:()=>t,reset:()=>{t=[]},push:s=>{t.push(s),e()}}}const W1=C.createContext(null);function I4({children:e}){const t=Et(),s=C.useCallback(f=>{const{nodes:m=[],setNodes:p,hasDefaultNodes:_,onNodesChange:y,nodeLookup:b,fitViewQueued:v,onNodesChangeMiddlewareMap:S}=t.getState();let w=m;for(const j of f)w=typeof j=="function"?j(w):j;let k=fx({items:w,lookup:b});for(const j of S.values())k=j(k);_&&p(w),k.length>0?y?.(k):v&&window.requestAnimationFrame(()=>{const{fitViewQueued:j,nodes:A,setNodes:N}=t.getState();j&&N(A)})},[]),a=hx(s),o=C.useCallback(f=>{const{edges:m=[],setEdges:p,hasDefaultEdges:_,onEdgesChange:y,edgeLookup:b}=t.getState();let v=m;for(const S of f)v=typeof S=="function"?S(v):S;_?p(v):y&&y(fx({items:v,lookup:b}))},[]),l=hx(o),c=C.useMemo(()=>({nodeQueue:a,edgeQueue:l}),[]);return d.jsx(W1.Provider,{value:c,children:e})}function Z4(){const e=C.useContext(W1);if(!e)throw new Error("useBatchContext must be used within a BatchProvider");return e}const P4=e=>!!e.panZoom;function Sg(){const e=D4(),t=Et(),s=Z4(),a=it(P4),o=C.useMemo(()=>{const l=y=>t.getState().nodeLookup.get(y),c=y=>{s.nodeQueue.push(y)},f=y=>{s.edgeQueue.push(y)},m=y=>{const{nodeLookup:b,nodeOrigin:v}=t.getState(),S=mx(y)?y:b.get(y.id),w=S.parentId?j1(S.position,S.measured,S.parentId,b,v):S.position,k={...S,position:w,width:S.measured?.width??S.width,height:S.measured?.height??S.height};return Fl(k,S.parentId?void 0:v)},p=(y,b,v={replace:!1})=>{c(S=>S.map(w=>{if(w.id===y){const k=typeof b=="function"?b(w):b;return v.replace&&mx(k)?k:{...w,...k}}return w}))},_=(y,b,v={replace:!1})=>{f(S=>S.map(w=>{if(w.id===y){const k=typeof b=="function"?b(w):b;return v.replace&&B4(k)?k:{...w,...k}}return w}))};return{getNodes:()=>t.getState().nodes.map(y=>({...y})),getNode:y=>l(y)?.internals.userNode,getInternalNode:l,getEdges:()=>{const{edges:y=[]}=t.getState();return y.map(b=>({...b}))},getEdge:y=>t.getState().edgeLookup.get(y),setNodes:c,setEdges:f,addNodes:y=>{const b=Array.isArray(y)?y:[y];s.nodeQueue.push(v=>[...v,...b])},addEdges:y=>{const b=Array.isArray(y)?y:[y];s.edgeQueue.push(v=>[...v,...b])},toObject:()=>{const{nodes:y=[],edges:b=[],transform:v}=t.getState(),[S,w,k]=v;return{nodes:y.map(j=>({...j})),edges:b.map(j=>({...j})),viewport:{x:S,y:w,zoom:k}}},deleteElements:async({nodes:y=[],edges:b=[]})=>{const{nodes:v,edges:S,onNodesDelete:w,onEdgesDelete:k,triggerNodeChanges:j,triggerEdgeChanges:A,onDelete:N,onBeforeDelete:T}=t.getState(),{nodes:E,edges:O}=await d3({nodesToRemove:y,edgesToRemove:b,nodes:v,edges:S,onBeforeDelete:T}),R=O.length>0,q=E.length>0;if(R){const Z=O.map(px);k?.(O),A(Z)}if(q){const Z=E.map(px);w?.(E),j(Z)}return(q||R)&&N?.({nodes:E,edges:O}),{deletedNodes:E,deletedEdges:O}},getIntersectingNodes:(y,b=!0,v)=>{const S=U0(y),w=S?y:m(y),k=v!==void 0;if(!w)return[];const{nodeOrigin:j}=t.getState();return(v||t.getState().nodes).filter(A=>{const N=t.getState().nodeLookup.get(A.id);if(N&&!S&&(A.id===y.id||!N.internals.positionAbsolute))return!1;const T=Fl(k?A:N,j),E=jd(T,w);return b&&E>0||E>=T.width*T.height||E>=w.width*w.height})},isNodeIntersecting:(y,b,v=!0)=>{const w=U0(y)?y:m(y);if(!w)return!1;const k=jd(w,b);return v&&k>0||k>=b.width*b.height||k>=w.width*w.height},updateNode:p,updateNodeData:(y,b,v={replace:!1})=>{p(y,S=>{const w=typeof b=="function"?b(S):b;return v.replace?{...S,data:w}:{...S,data:{...S.data,...w}}},v)},updateEdge:_,updateEdgeData:(y,b,v={replace:!1})=>{_(y,S=>{const w=typeof b=="function"?b(S):b;return v.replace?{...S,data:w}:{...S,data:{...S.data,...w}}},v)},getNodesBounds:y=>{const{nodeLookup:b,nodeOrigin:v}=t.getState();return r3(y,{nodeLookup:b,nodeOrigin:v})},getHandleConnections:({type:y,id:b,nodeId:v})=>Array.from(t.getState().connectionLookup.get(`${v}-${y}${b?`-${b}`:""}`)?.values()??[]),getNodeConnections:({type:y,handleId:b,nodeId:v})=>Array.from(t.getState().connectionLookup.get(`${v}${y?b?`-${y}-${b}`:`-${y}`:""}`)?.values()??[]),fitView:async y=>{const b=t.getState().fitViewResolver??m3();return t.setState({fitViewQueued:!0,fitViewOptions:y,fitViewResolver:b}),s.nodeQueue.push(v=>[...v]),b.promise}}},[]);return C.useMemo(()=>({...o,...e,viewportInitialized:a}),[a])}const gx=e=>e.selected,Y4=typeof window<"u"?window:void 0;function V4({deleteKeyCode:e,multiSelectionKeyCode:t}){const s=Et(),{deleteElements:a}=Sg(),o=Ql(e,{actInsideInputWithModifier:!1}),l=Ql(t,{target:Y4});C.useEffect(()=>{if(o){const{edges:c,nodes:f}=s.getState();a({nodes:f.filter(gx),edges:c.filter(gx)}),s.setState({nodesSelectionActive:!1})}},[o]),C.useEffect(()=>{s.setState({multiSelectionActive:l})},[l])}function K4(e){const t=Et();C.useEffect(()=>{const s=()=>{if(!e.current||!(e.current.checkVisibility?.()??!0))return!1;const a=yg(e.current);(a.height===0||a.width===0)&&t.getState().onError?.("004",Ds.error004()),t.setState({width:a.width||500,height:a.height||500})};if(e.current){s(),window.addEventListener("resize",s);const a=new ResizeObserver(()=>s());return a.observe(e.current),()=>{window.removeEventListener("resize",s),a.disconnect()}}},[])}const of={position:"absolute",width:"100%",height:"100%",top:0,left:0},G4=e=>({userSelectionActive:e.userSelectionActive,lib:e.lib,connectionInProgress:e.connection.inProgress});function X4({onPaneContextMenu:e,zoomOnScroll:t=!0,zoomOnPinch:s=!0,panOnScroll:a=!1,panActivationKeyPressed:o,panOnScrollSpeed:l=.5,panOnScrollMode:c=so.Free,zoomOnDoubleClick:f=!0,panOnDrag:m=!0,defaultViewport:p,translateExtent:_,minZoom:y,maxZoom:b,zoomActivationKeyCode:v,preventScrolling:S=!0,children:w,noWheelClassName:k,noPanClassName:j,onViewportChange:A,isControlledViewport:N,paneClickDistance:T,selectionOnDrag:E}){const O=Et(),R=C.useRef(null),{userSelectionActive:q,lib:Z,connectionInProgress:oe}=it(G4,Lt),Q=Ql(v),ee=C.useRef();K4(R);const X=C.useCallback(L=>{A?.({x:L[0],y:L[1],zoom:L[2]}),N||O.setState({transform:L})},[A,N]);return C.useEffect(()=>{if(R.current){ee.current=Q3({domNode:R.current,minZoom:y,maxZoom:b,translateExtent:_,viewport:p,onDraggingChange:U=>O.setState(B=>B.paneDragging===U?B:{paneDragging:U}),onPanZoomStart:(U,B)=>{const{onViewportChangeStart:V,onMoveStart:te}=O.getState();te?.(U,B),V?.(B)},onPanZoom:(U,B)=>{const{onViewportChange:V,onMove:te}=O.getState();te?.(U,B),V?.(B)},onPanZoomEnd:(U,B)=>{const{onViewportChangeEnd:V,onMoveEnd:te}=O.getState();te?.(U,B),V?.(B)}});const{x:L,y:I,zoom:D}=ee.current.getViewport();return O.setState({panZoom:ee.current,transform:[L,I,D],domNode:R.current.closest(".react-flow")}),()=>{ee.current?.destroy()}}},[]),C.useEffect(()=>{ee.current?.update({onPaneContextMenu:e,zoomOnScroll:t,zoomOnPinch:s,panOnScroll:a,panActivationKeyPressed:o,panOnScrollSpeed:l,panOnScrollMode:c,zoomOnDoubleClick:f,panOnDrag:m,zoomActivationKeyPressed:Q,preventScrolling:S,noPanClassName:j,userSelectionActive:q,noWheelClassName:k,lib:Z,onTransformChange:X,connectionInProgress:oe,selectionOnDrag:E,paneClickDistance:T})},[e,t,s,a,o,l,c,f,m,Q,S,j,q,k,Z,X,oe,E,T]),d.jsx("div",{className:"react-flow__renderer",ref:R,style:of,children:w})}const F4=e=>({userSelectionActive:e.userSelectionActive,userSelectionRect:e.userSelectionRect});function J4(){const{userSelectionActive:e,userSelectionRect:t}=it(F4,Lt);return e&&t?d.jsx("div",{className:"react-flow__selection react-flow__container",style:{width:t.width,height:t.height,transform:`translate(${t.x}px, ${t.y}px)`}}):null}const Pm=(e,t)=>s=>{s.target===t.current&&e?.(s)},Q4=e=>({userSelectionActive:e.userSelectionActive,elementsSelectable:e.elementsSelectable,dragging:e.paneDragging,panBy:e.panBy,autoPanSpeed:e.autoPanSpeed});function W4({isSelecting:e,selectionKeyPressed:t,selectionMode:s=Gl.Full,panOnDrag:a,autoPanOnSelection:o,paneClickDistance:l,selectionOnDrag:c,onSelectionStart:f,onSelectionEnd:m,onPaneClick:p,onPaneContextMenu:_,onPaneScroll:y,onPaneMouseEnter:b,onPaneMouseMove:v,onPaneMouseLeave:S,children:w}){const k=C.useRef(0),j=Et(),{userSelectionActive:A,elementsSelectable:N,dragging:T,panBy:E,autoPanSpeed:O}=it(Q4,Lt),R=N&&(e||A),q=C.useRef(null),Z=C.useRef(),oe=C.useRef(new Set),Q=C.useRef(new Set),ee=C.useRef(!1),X=C.useRef(!1),L=C.useRef({x:0,y:0}),I=C.useRef(!1),D=P=>{if(X.current||ee.current||j.getState().connection.inProgress){X.current=!1,ee.current=!1;return}const{connectionClickStartHandle:ae,connection:Se,onClickConnectEnd:je,resetSelectedElements:we}=j.getState();if(ae){const ve=structuredClone(Se);delete ve.inProgress,ve.toPosition=ve.toHandle?ve.toHandle.position:null,je?.(P.nativeEvent,ve),j.setState({connectionClickStartHandle:null})}p?.(P),we(),j.setState({nodesSelectionActive:!1})},U=P=>{if(Array.isArray(a)&&a?.includes(2)){P.preventDefault();return}_?.(P)},B=y?P=>y(P):void 0,V=P=>{X.current&&(P.stopPropagation(),X.current=!1)},te=P=>{if(P.pointerType==="touch"&&a!==!1&&!t)return;const{domNode:ae,transform:Se}=j.getState();if(Z.current=ae?.getBoundingClientRect(),!Z.current)return;const je=P.target===q.current;if(!je&&!!P.target.closest(".nokey")||!e||!(c&&je||t)||P.button!==0||!P.isPrimary)return;P.target?.setPointerCapture?.(P.pointerId),X.current=!1;const{x:xe,y:ke}=zs(P.nativeEvent,Z.current),me=fc({x:xe,y:ke},Se);j.setState({userSelectionRect:{width:0,height:0,startX:me.x,startY:me.y,x:xe,y:ke}}),je||(P.stopPropagation(),P.preventDefault())};function z(P,ae){const{userSelectionRect:Se}=j.getState();if(!Se)return;const{transform:je,nodeLookup:we,edgeLookup:ve,connectionLookup:xe,triggerNodeChanges:ke,triggerEdgeChanges:me,defaultEdgeOptions:Ge}=j.getState(),Ke={x:Se.startX,y:Se.startY},{x:rt,y:nt}=Er(Ke,je),qe={startX:Ke.x,startY:Ke.y,x:P<rt?P:rt,y:ae<nt?ae:nt,width:Math.abs(P-rt),height:Math.abs(ae-nt)},Xt=oe.current,Pt=Q.current;oe.current=new Set(mg(we,qe,je,s===Gl.Partial,!0).map(St=>St.id)),Q.current=new Set;const lt=Ge?.selectable??!0;for(const St of oe.current){const mn=xe.get(St);if(mn)for(const{edgeId:Tt}of mn.values()){const Ft=ve.get(Tt);Ft&&(Ft.selectable??lt)&&Q.current.add(Tt)}}if(!B0(Xt,oe.current)){const St=cr(we,oe.current);ke(St)}if(!B0(Pt,Q.current)){const St=cr(ve,Q.current);me(St)}j.setState({userSelectionRect:qe,userSelectionActive:!0,nodesSelectionActive:!1})}function Y(){if(!o||!Z.current)return;const[P,ae]=hg(L.current,Z.current,O);E({x:P,y:ae}).then(Se=>{if(!X.current||!Se){k.current=requestAnimationFrame(Y);return}const{x:je,y:we}=L.current;z(je,we),k.current=requestAnimationFrame(Y)})}const ce=()=>{cancelAnimationFrame(k.current),k.current=0,I.current=!1};C.useEffect(()=>()=>ce(),[]);const de=P=>{const{userSelectionRect:ae,transform:Se,resetSelectedElements:je}=j.getState();if(!Z.current||!ae)return;const{x:we,y:ve}=zs(P.nativeEvent,Z.current);L.current={x:we,y:ve};const xe=Er({x:ae.startX,y:ae.startY},Se);if(!X.current){const ke=t?0:l;if(Math.hypot(we-xe.x,ve-xe.y)<=ke)return;je(),f?.(P)}X.current=!0,I.current||(Y(),I.current=!0),z(we,ve)},_e=P=>{if(!R){P.target===q.current&&j.getState().connection.inProgress&&(ee.current=!0);return}P.button===0&&(P.target?.releasePointerCapture?.(P.pointerId),!A&&P.target===q.current&&j.getState().userSelectionRect&&D?.(P),j.setState({userSelectionActive:!1,userSelectionRect:null}),X.current&&(m?.(P),j.setState({nodesSelectionActive:oe.current.size>0})),ce())},F=P=>{P.target?.releasePointerCapture?.(P.pointerId),ce()},G=a===!0||Array.isArray(a)&&a.includes(0);return d.jsxs("div",{className:an(["react-flow__pane",{draggable:G,dragging:T,selection:e}]),onClick:R?void 0:Pm(D,q),onContextMenu:Pm(U,q),onWheel:Pm(B,q),onPointerEnter:R?void 0:b,onPointerMove:R?de:v,onPointerUp:_e,onPointerCancel:R?F:void 0,onPointerDownCapture:R?te:void 0,onClickCapture:R?V:void 0,onPointerLeave:S,ref:q,style:of,children:[w,d.jsx(J4,{})]})}function xh({id:e,store:t,unselect:s=!1,nodeRef:a}){const{addSelectedNodes:o,unselectNodesAndEdges:l,multiSelectionActive:c,nodeLookup:f,onError:m}=t.getState(),p=f.get(e);if(!p){m?.("012",Ds.error012(e));return}t.setState({nodesSelectionActive:!1}),p.selected?(s||p.selected&&c)&&(l({nodes:[p],edges:[]}),requestAnimationFrame(()=>a?.current?.blur())):o([e])}function ek({nodeRef:e,disabled:t=!1,noDragClassName:s,handleSelector:a,nodeId:o,isSelectable:l,nodeClickDistance:c}){const f=Et(),[m,p]=C.useState(!1),_=C.useRef();return C.useEffect(()=>{if(!t)return _.current=q3({getStoreItems:()=>f.getState(),onNodeMouseDown:y=>{xh({id:y,store:f,nodeRef:e})},onDragStart:()=>{p(!0)},onDragStop:()=>{p(!1)}}),()=>{_.current?.destroy(),_.current=void 0}},[t,f,e]),C.useEffect(()=>{t||!e.current||!_.current||_.current.update({noDragClassName:s,handleSelector:a,domNode:e.current,isSelectable:l,nodeId:o,nodeClickDistance:c})},[s,a,t,l,e,o,c]),m}const eL=e=>t=>t.selected&&(t.draggable||e&&typeof t.draggable>"u");function tk(){const e=Et();return C.useCallback(s=>{const{nodeExtent:a,snapToGrid:o,snapGrid:l,nodesDraggable:c,onError:f,updateNodePositions:m,nodeLookup:p,nodeOrigin:_}=e.getState(),y=new Map,b=eL(c),v=o?l[0]:5,S=o?l[1]:5,w=s.direction.x*v*s.factor,k=s.direction.y*S*s.factor;for(const[,j]of p){if(!b(j))continue;let A={x:j.internals.positionAbsolute.x+w,y:j.internals.positionAbsolute.y+k};o&&(A=dc(A,l));const{position:N,positionAbsolute:T}=b1({nodeId:j.id,nextPosition:A,nodeLookup:p,nodeExtent:a,nodeOrigin:_,onError:f});j.position=N,j.internals.positionAbsolute=T,y.set(j.id,j)}m(y)},[])}const kg=C.createContext(null),tL=kg.Provider;kg.Consumer;const nk=()=>C.useContext(kg),nL=e=>({connectOnClick:e.connectOnClick,noPanClassName:e.noPanClassName,rfId:e.rfId}),sk=C.createContext(null);function sL({children:e}){const t=it(nL,Lt);return d.jsx(sk.Provider,{value:t,children:e})}function iL(){const e=C.useContext(sk);if(!e)throw new Error("useHandleConfig must be used within a HandleConfigProvider");return e}const aL={connectingFrom:!1,connectingTo:!1,clickConnecting:!1,isPossibleEndHandle:!0,connectionInProcess:!1,clickConnectionInProcess:!1,valid:!1},oL=(e,t,s)=>a=>{const{connectionClickStartHandle:o,connectionMode:l,connection:c}=a,{fromHandle:f,toHandle:m,isValid:p}=c;if(!f&&!o)return aL;const _=m?.nodeId===e&&m?.id===t&&m?.type===s;return{connectingFrom:f?.nodeId===e&&f?.id===t&&f?.type===s,connectingTo:_,clickConnecting:o?.nodeId===e&&o?.id===t&&o?.type===s,isPossibleEndHandle:l===jr.Strict?f?.type!==s:e!==f?.nodeId||t!==f?.id,connectionInProcess:!!f,clickConnectionInProcess:!!o,valid:_&&p}};function rL({type:e="source",position:t=ze.Top,isValidConnection:s,isConnectable:a=!0,isConnectableStart:o=!0,isConnectableEnd:l=!0,id:c,onConnect:f,children:m,className:p,onMouseDown:_,onTouchStart:y,...b},v){const S=c||null,w=e==="target",k=Et(),j=nk(),{connectOnClick:A,noPanClassName:N,rfId:T}=iL(),{connectingFrom:E,connectingTo:O,clickConnecting:R,isPossibleEndHandle:q,connectionInProcess:Z,clickConnectionInProcess:oe,valid:Q}=it(oL(j,S,e),Lt);j||k.getState().onError?.("010",Ds.error010());const ee=I=>{const{defaultEdgeOptions:D,onConnect:U,hasDefaultEdges:B}=k.getState(),V={...D,...I};if(B){const{edges:te,setEdges:z,onError:Y}=k.getState();z(U4(V,te,{onError:Y}))}U?.(V),f?.(V)},X=I=>{if(!j)return;const D=C1(I.nativeEvent);if(o&&(D&&I.button===0||!D)){const U=k.getState();vh.onPointerDown(I.nativeEvent,{handleDomNode:I.currentTarget,autoPanOnConnect:U.autoPanOnConnect,connectionMode:U.connectionMode,connectionRadius:U.connectionRadius,domNode:U.domNode,nodeLookup:U.nodeLookup,lib:U.lib,isTarget:w,handleId:S,nodeId:j,flowId:U.rfId,panBy:U.panBy,cancelConnection:U.cancelConnection,onConnectStart:U.onConnectStart,onConnectEnd:(...B)=>k.getState().onConnectEnd?.(...B),updateConnection:U.updateConnection,onConnect:ee,isValidConnection:s||((...B)=>k.getState().isValidConnection?.(...B)??!0),getTransform:()=>k.getState().transform,getFromHandle:()=>k.getState().connection.fromHandle,autoPanSpeed:U.autoPanSpeed,dragThreshold:U.connectionDragThreshold})}D?_?.(I):y?.(I)},L=I=>{const{onClickConnectStart:D,onClickConnectEnd:U,connectionClickStartHandle:B,connectionMode:V,isValidConnection:te,lib:z,rfId:Y,nodeLookup:ce,connection:de}=k.getState();if(!j||!B&&!o)return;if(!B){D?.(I.nativeEvent,{nodeId:j,handleId:S,handleType:e}),k.setState({connectionClickStartHandle:{nodeId:j,type:e,id:S}});return}const _e=E1(I.target),F=s||te,{connection:G,isValid:P}=vh.isValid(I.nativeEvent,{handle:{nodeId:j,id:S,type:e},connectionMode:V,fromNodeId:B.nodeId,fromHandleId:B.id||null,fromType:B.type,isValidConnection:F,flowId:Y,doc:_e,lib:z,nodeLookup:ce});P&&G&&ee(G);const ae=structuredClone(de);delete ae.inProgress,ae.toPosition=ae.toHandle?ae.toHandle.position:null,U?.(I,ae),k.setState({connectionClickStartHandle:null})};return d.jsx("div",{"data-handleid":S,"data-nodeid":j,"data-handlepos":t,"data-id":`${T}-${j}-${S}-${e}`,className:an(["react-flow__handle",`react-flow__handle-${t}`,"nodrag",N,p,{source:!w,target:w,connectable:a,connectablestart:o,connectableend:l,clickconnecting:R,connectingfrom:E,connectingto:O,valid:Q,connectionindicator:a&&(!Z||q)&&(Z||oe?l:o)}]),onMouseDown:X,onTouchStart:X,onClick:A?L:void 0,ref:v,...b,children:m})}const Ar=C.memo(J1(rL));function lL({data:e,isConnectable:t,sourcePosition:s=ze.Bottom}){return d.jsxs(d.Fragment,{children:[e?.label,d.jsx(Ar,{type:"source",position:s,isConnectable:t})]})}function cL({data:e,isConnectable:t,targetPosition:s=ze.Top,sourcePosition:a=ze.Bottom}){return d.jsxs(d.Fragment,{children:[d.jsx(Ar,{type:"target",position:s,isConnectable:t}),e?.label,d.jsx(Ar,{type:"source",position:a,isConnectable:t})]})}function uL(){return null}function dL({data:e,isConnectable:t,targetPosition:s=ze.Top}){return d.jsxs(d.Fragment,{children:[d.jsx(Ar,{type:"target",position:s,isConnectable:t}),e?.label]})}const Nd={ArrowUp:{x:0,y:-1},ArrowDown:{x:0,y:1},ArrowLeft:{x:-1,y:0},ArrowRight:{x:1,y:0}},yx={input:lL,default:cL,output:dL,group:uL};function fL(e){return e.internals.handleBounds===void 0?{width:e.width??e.initialWidth??e.style?.width,height:e.height??e.initialHeight??e.style?.height}:{width:e.width??e.style?.width,height:e.height??e.style?.height}}const pL=e=>{const{width:t,height:s,x:a,y:o}=uc(e.nodeLookup,{filter:l=>!!l.selected});return{width:Ms(t)?t:null,height:Ms(s)?s:null,userSelectionActive:e.userSelectionActive,transformString:`translate(${e.transform[0]}px,${e.transform[1]}px) scale(${e.transform[2]}) translate(${a}px,${o}px)`}};function mL({onSelectionContextMenu:e,noPanClassName:t,disableKeyboardA11y:s}){const a=Et(),{width:o,height:l,transformString:c,userSelectionActive:f}=it(pL,Lt),m=tk(),p=C.useRef(null);C.useEffect(()=>{s||p.current?.focus({preventScroll:!0})},[s]);const _=!f&&o!==null&&l!==null;if(ek({nodeRef:p,disabled:!_}),!_)return null;const y=e?v=>{const S=a.getState().nodes.filter(w=>w.selected);e(v,S)}:void 0,b=v=>{Object.prototype.hasOwnProperty.call(Nd,v.key)&&(v.preventDefault(),m({direction:Nd[v.key],factor:v.shiftKey?4:1}))};return d.jsx("div",{className:an(["react-flow__nodesselection","react-flow__container",t]),style:{transform:c},children:d.jsx("div",{ref:p,className:"react-flow__nodesselection-rect",onContextMenu:y,tabIndex:s?void 0:-1,onKeyDown:s?void 0:b,style:{width:o,height:l}})})}const _x=typeof window<"u"?window:void 0,hL=e=>({nodesSelectionActive:e.nodesSelectionActive,userSelectionActive:e.userSelectionActive});function ik({children:e,onPaneClick:t,onPaneMouseEnter:s,onPaneMouseMove:a,onPaneMouseLeave:o,onPaneContextMenu:l,onPaneScroll:c,paneClickDistance:f,deleteKeyCode:m,selectionKeyCode:p,selectionOnDrag:_,selectionMode:y,onSelectionStart:b,onSelectionEnd:v,multiSelectionKeyCode:S,panActivationKeyCode:w,zoomActivationKeyCode:k,elementsSelectable:j,zoomOnScroll:A,zoomOnPinch:N,panOnScroll:T,panOnScrollSpeed:E,panOnScrollMode:O,zoomOnDoubleClick:R,panOnDrag:q,autoPanOnSelection:Z,defaultViewport:oe,translateExtent:Q,minZoom:ee,maxZoom:X,preventScrolling:L,onSelectionContextMenu:I,noWheelClassName:D,noPanClassName:U,disableKeyboardA11y:B,onViewportChange:V,isControlledViewport:te}){const{nodesSelectionActive:z,userSelectionActive:Y}=it(hL,Lt),ce=Ql(p,{target:_x}),de=Ql(w,{target:_x}),_e=de||q,F=de||T,G=_&&_e!==!0,P=ce||Y||G;return V4({deleteKeyCode:m,multiSelectionKeyCode:S}),d.jsx(X4,{onPaneContextMenu:l,elementsSelectable:j,zoomOnScroll:A,zoomOnPinch:N,panOnScroll:F,panActivationKeyPressed:de,panOnScrollSpeed:E,panOnScrollMode:O,zoomOnDoubleClick:R,panOnDrag:!ce&&_e,defaultViewport:oe,translateExtent:Q,minZoom:ee,maxZoom:X,zoomActivationKeyCode:k,preventScrolling:L,noWheelClassName:D,noPanClassName:U,onViewportChange:V,isControlledViewport:te,paneClickDistance:f,selectionOnDrag:G,children:d.jsxs(W4,{onSelectionStart:b,onSelectionEnd:v,onPaneClick:t,onPaneMouseEnter:s,onPaneMouseMove:a,onPaneMouseLeave:o,onPaneContextMenu:l,onPaneScroll:c,panOnDrag:_e,autoPanOnSelection:Z,isSelecting:!!P,selectionMode:y,selectionKeyPressed:ce,paneClickDistance:f,selectionOnDrag:G,children:[e,z&&d.jsx(mL,{onSelectionContextMenu:I,noPanClassName:U,disableKeyboardA11y:B})]})})}ik.displayName="FlowRenderer";const gL=C.memo(ik),yL=e=>t=>e?mg(t.nodeLookup,{x:0,y:0,width:t.width,height:t.height},t.transform,!0).map(s=>s.id):Array.from(t.nodeLookup.keys());function _L(e){return it(C.useCallback(yL(e),[e]),Lt)}const bL=e=>e.updateNodeInternals;function vL(){const e=it(bL),[t]=C.useState(()=>typeof ResizeObserver>"u"?null:new ResizeObserver(s=>{const a=new Map;s.forEach(o=>{const l=o.target.getAttribute("data-id");a.set(l,{id:l,nodeElement:o.target,force:!0})}),e(a)}));return C.useEffect(()=>()=>{t?.disconnect()},[t]),t}function xL({node:e,nodeType:t,hasDimensions:s,resizeObserver:a}){const o=Et(),l=C.useRef(null),c=C.useRef(null),f=C.useRef(e.sourcePosition),m=C.useRef(e.targetPosition),p=C.useRef(t),_=s&&!!e.internals.handleBounds;return C.useEffect(()=>{l.current&&!e.hidden&&(!_||c.current!==l.current)&&(c.current&&a?.unobserve(c.current),a?.observe(l.current),c.current=l.current)},[_,e.hidden]),C.useEffect(()=>()=>{c.current&&(a?.unobserve(c.current),c.current=null)},[]),C.useEffect(()=>{if(l.current){const y=p.current!==t,b=f.current!==e.sourcePosition,v=m.current!==e.targetPosition;(y||b||v)&&(p.current=t,f.current=e.sourcePosition,m.current=e.targetPosition,o.getState().updateNodeInternals(new Map([[e.id,{id:e.id,nodeElement:l.current,force:!0}]])))}},[e.id,t,e.sourcePosition,e.targetPosition]),l}function wL({id:e,onClick:t,onMouseEnter:s,onMouseMove:a,onMouseLeave:o,onContextMenu:l,onDoubleClick:c,nodesDraggable:f,elementsSelectable:m,nodesConnectable:p,nodesFocusable:_,resizeObserver:y,noDragClassName:b,noPanClassName:v,disableKeyboardA11y:S,rfId:w,nodeTypes:k,nodeClickDistance:j,onError:A}){const{node:N,internals:T,isParent:E}=it(P=>{const ae=P.nodeLookup.get(e),Se=P.parentLookup.has(e);return{node:ae,internals:ae.internals,isParent:Se}},Lt);let O=N.type||"default",R=k?.[O]||yx[O];R===void 0&&(A?.("003",Ds.error003(O)),O="default",R=k?.default||yx.default);const q=!!(N.draggable||f&&typeof N.draggable>"u"),Z=!!(N.selectable||m&&typeof N.selectable>"u"),oe=!!(N.connectable||p&&typeof N.connectable>"u"),Q=!!(N.focusable||_&&typeof N.focusable>"u"),ee=Et(),X=k1(N),L=xL({node:N,nodeType:O,hasDimensions:X,resizeObserver:y}),I=ek({nodeRef:L,disabled:N.hidden||!q,noDragClassName:b,handleSelector:N.dragHandle,nodeId:e,isSelectable:Z,nodeClickDistance:j}),D=tk();if(N.hidden)return null;const U=qs(N),B=fL(N),V=Z||q||t||s||a||o,te=s?P=>s(P,{...T.userNode}):void 0,z=a?P=>a(P,{...T.userNode}):void 0,Y=o?P=>o(P,{...T.userNode}):void 0,ce=l?P=>l(P,{...T.userNode}):void 0,de=c?P=>c(P,{...T.userNode}):void 0,_e=P=>{const{selectNodesOnDrag:ae,nodeDragThreshold:Se}=ee.getState();Z&&(!ae||!q||Se>0)&&xh({id:e,store:ee,nodeRef:L}),t&&t(P,{...T.userNode})},F=P=>{if(!(T1(P.nativeEvent)||S)){if(h1.includes(P.key)&&Z){const ae=P.key==="Escape";xh({id:e,store:ee,unselect:ae,nodeRef:L})}else if(q&&N.selected&&Object.prototype.hasOwnProperty.call(Nd,P.key)){P.preventDefault();const{ariaLabelConfig:ae}=ee.getState();ee.setState({ariaLiveMessage:ae["node.a11yDescription.ariaLiveMessage"]({direction:P.key.replace("Arrow","").toLowerCase(),x:~~T.positionAbsolute.x,y:~~T.positionAbsolute.y})}),D({direction:Nd[P.key],factor:P.shiftKey?4:1})}}},G=()=>{if(S||!L.current?.matches(":focus-visible"))return;const{transform:P,width:ae,height:Se,autoPanOnNodeFocus:je,setCenter:we}=ee.getState();if(!je)return;mg(new Map([[e,N]]),{x:0,y:0,width:ae,height:Se},P,!0).length>0||we(N.position.x+U.width/2,N.position.y+U.height/2,{zoom:P[2]})};return d.jsx("div",{className:an(["react-flow__node",`react-flow__node-${O}`,{[v]:q},N.className,{selected:N.selected,selectable:Z,parent:E,draggable:q,dragging:I}]),ref:L,style:{zIndex:T.z,transform:`translate(${T.positionAbsolute.x}px,${T.positionAbsolute.y}px)`,pointerEvents:V?"all":"none",visibility:X?"visible":"hidden",...N.style,...B},"data-id":e,"data-testid":`rf__node-${e}`,onMouseEnter:te,onMouseMove:z,onMouseLeave:Y,onContextMenu:ce,onClick:_e,onDoubleClick:de,onKeyDown:Q?F:void 0,tabIndex:Q?0:void 0,onFocus:Q?G:void 0,role:N.ariaRole??(Q?"group":void 0),"aria-roledescription":"node","aria-describedby":S?void 0:`${K1}-${w}`,"aria-label":N.ariaLabel,...N.domAttributes,children:d.jsx(tL,{value:e,children:d.jsx(R,{id:e,data:N.data,type:O,positionAbsoluteX:T.positionAbsolute.x,positionAbsoluteY:T.positionAbsolute.y,selected:N.selected??!1,selectable:Z,draggable:q,deletable:N.deletable??!0,isConnectable:oe,sourcePosition:N.sourcePosition,targetPosition:N.targetPosition,dragging:I,dragHandle:N.dragHandle,zIndex:T.z,parentId:N.parentId,...U})})})}var SL=C.memo(wL);const kL=e=>({nodesConnectable:e.nodesConnectable,nodesFocusable:e.nodesFocusable,elementsSelectable:e.elementsSelectable,onError:e.onError});function ak(e){const{nodesConnectable:t,nodesFocusable:s,elementsSelectable:a,onError:o}=it(kL,Lt),l=_L(e.onlyRenderVisibleElements),c=vL();return d.jsx("div",{className:"react-flow__nodes",style:of,children:l.map(f=>d.jsx(SL,{id:f,nodeTypes:e.nodeTypes,nodeExtent:e.nodeExtent,onClick:e.onNodeClick,onMouseEnter:e.onNodeMouseEnter,onMouseMove:e.onNodeMouseMove,onMouseLeave:e.onNodeMouseLeave,onContextMenu:e.onNodeContextMenu,onDoubleClick:e.onNodeDoubleClick,noDragClassName:e.noDragClassName,noPanClassName:e.noPanClassName,rfId:e.rfId,disableKeyboardA11y:e.disableKeyboardA11y,resizeObserver:c,nodesDraggable:e.nodesDraggable??!0,nodesConnectable:t,nodesFocusable:s,elementsSelectable:a,nodeClickDistance:e.nodeClickDistance,onError:o},f))})}ak.displayName="NodeRenderer";const jL=C.memo(ak);function NL(e){return it(C.useCallback(s=>{if(!e)return s.edges.map(o=>o.id);const a=[];if(s.width&&s.height)for(const o of s.edges){const l=s.nodeLookup.get(o.source),c=s.nodeLookup.get(o.target);l&&c&&_3({sourceNode:l,targetNode:c,width:s.width,height:s.height,transform:s.transform})&&a.push(o.id)}return a},[e]),Lt)}const EL=({color:e="none",strokeWidth:t=1})=>{const s={strokeWidth:t,...e&&{stroke:e}};return d.jsx("polyline",{className:"arrow",style:s,strokeLinecap:"round",fill:"none",strokeLinejoin:"round",points:"-5,-4 0,0 -5,4"})},TL=({color:e="none",strokeWidth:t=1})=>{const s={strokeWidth:t,...e&&{stroke:e,fill:e}};return d.jsx("polyline",{className:"arrowclosed",style:s,strokeLinecap:"round",strokeLinejoin:"round",points:"-5,-4 0,0 -5,4 -5,-4"})},bx={[Xl.Arrow]:EL,[Xl.ArrowClosed]:TL};function CL(e){const t=Et();return C.useMemo(()=>Object.prototype.hasOwnProperty.call(bx,e)?bx[e]:(t.getState().onError?.("009",Ds.error009(e)),null),[e])}const AL=({id:e,type:t,color:s,width:a=12.5,height:o=12.5,markerUnits:l="strokeWidth",strokeWidth:c,orient:f="auto-start-reverse"})=>{const m=CL(t);return m?d.jsx("marker",{className:"react-flow__arrowhead",id:e,markerWidth:`${a}`,markerHeight:`${o}`,viewBox:"-10 -10 20 20",markerUnits:l,orient:f,refX:"0",refY:"0",children:d.jsx(m,{color:s,strokeWidth:c})}):null},ok=({defaultColor:e,rfId:t})=>{const s=it(l=>l.edges),a=it(l=>l.defaultEdgeOptions),o=C.useMemo(()=>E3(s,{id:t,defaultColor:e,defaultMarkerStart:a?.markerStart,defaultMarkerEnd:a?.markerEnd}),[s,a,t,e]);return o.length?d.jsx("svg",{className:"react-flow__marker","aria-hidden":"true",children:d.jsx("defs",{children:o.map(l=>d.jsx(AL,{id:l.id,type:l.type,color:l.color,width:l.width,height:l.height,markerUnits:l.markerUnits,strokeWidth:l.strokeWidth,orient:l.orient},l.id))})}):null};ok.displayName="MarkerDefinitions";var OL=C.memo(ok);function rk({x:e,y:t,label:s,labelStyle:a,labelShowBg:o=!0,labelBgStyle:l,labelBgPadding:c=[2,4],labelBgBorderRadius:f=2,children:m,className:p,..._}){const[y,b]=C.useState({x:1,y:0,width:0,height:0}),v=an(["react-flow__edge-textwrapper",p]),S=C.useRef(null);return C.useEffect(()=>{if(S.current){const w=S.current.getBBox();b({x:w.x,y:w.y,width:w.width,height:w.height})}},[s]),s?d.jsxs("g",{transform:`translate(${e-y.width/2} ${t-y.height/2})`,className:v,visibility:y.width?"visible":"hidden",..._,children:[o&&d.jsx("rect",{width:y.width+2*c[0],x:-c[0],y:-c[1],height:y.height+2*c[1],className:"react-flow__edge-textbg",style:l,rx:f,ry:f}),d.jsx("text",{className:"react-flow__edge-text",y:y.height/2,dy:"0.3em",ref:S,style:a,children:s}),m]}):null}rk.displayName="EdgeText";const ML=C.memo(rk);function rf({path:e,labelX:t,labelY:s,label:a,labelStyle:o,labelShowBg:l,labelBgStyle:c,labelBgPadding:f,labelBgBorderRadius:m,interactionWidth:p=20,..._}){return d.jsxs(d.Fragment,{children:[d.jsx("path",{..._,d:e,fill:"none",className:an(["react-flow__edge-path",_.className])}),p?d.jsx("path",{d:e,fill:"none",strokeOpacity:0,strokeWidth:p,className:"react-flow__edge-interaction"}):null,a&&Ms(t)&&Ms(s)?d.jsx(ML,{x:t,y:s,label:a,labelStyle:o,labelShowBg:l,labelBgStyle:c,labelBgPadding:f,labelBgBorderRadius:m}):null]})}function vx({pos:e,x1:t,y1:s,x2:a,y2:o}){return e===ze.Left||e===ze.Right?[.5*(t+a),s]:[t,.5*(s+o)]}function lk({sourceX:e,sourceY:t,sourcePosition:s=ze.Bottom,targetX:a,targetY:o,targetPosition:l=ze.Top}){const[c,f]=vx({pos:s,x1:e,y1:t,x2:a,y2:o}),[m,p]=vx({pos:l,x1:a,y1:o,x2:e,y2:t}),[_,y,b,v]=A1({sourceX:e,sourceY:t,targetX:a,targetY:o,sourceControlX:c,sourceControlY:f,targetControlX:m,targetControlY:p});return[`M${e},${t} C${c},${f} ${m},${p} ${a},${o}`,_,y,b,v]}function ck(e){return C.memo(({id:t,sourceX:s,sourceY:a,targetX:o,targetY:l,sourcePosition:c,targetPosition:f,label:m,labelStyle:p,labelShowBg:_,labelBgStyle:y,labelBgPadding:b,labelBgBorderRadius:v,style:S,markerEnd:w,markerStart:k,interactionWidth:j})=>{const[A,N,T]=lk({sourceX:s,sourceY:a,sourcePosition:c,targetX:o,targetY:l,targetPosition:f}),E=e.isInternal?void 0:t;return d.jsx(rf,{id:E,path:A,labelX:N,labelY:T,label:m,labelStyle:p,labelShowBg:_,labelBgStyle:y,labelBgPadding:b,labelBgBorderRadius:v,style:S,markerEnd:w,markerStart:k,interactionWidth:j})})}const zL=ck({isInternal:!1}),uk=ck({isInternal:!0});zL.displayName="SimpleBezierEdge";uk.displayName="SimpleBezierEdgeInternal";function dk(e){return C.memo(({id:t,sourceX:s,sourceY:a,targetX:o,targetY:l,label:c,labelStyle:f,labelShowBg:m,labelBgStyle:p,labelBgPadding:_,labelBgBorderRadius:y,style:b,sourcePosition:v=ze.Bottom,targetPosition:S=ze.Top,markerEnd:w,markerStart:k,pathOptions:j,interactionWidth:A})=>{const[N,T,E]=yh({sourceX:s,sourceY:a,sourcePosition:v,targetX:o,targetY:l,targetPosition:S,borderRadius:j?.borderRadius,offset:j?.offset,stepPosition:j?.stepPosition}),O=e.isInternal?void 0:t;return d.jsx(rf,{id:O,path:N,labelX:T,labelY:E,label:c,labelStyle:f,labelShowBg:m,labelBgStyle:p,labelBgPadding:_,labelBgBorderRadius:y,style:b,markerEnd:w,markerStart:k,interactionWidth:A})})}const fk=dk({isInternal:!1}),pk=dk({isInternal:!0});fk.displayName="SmoothStepEdge";pk.displayName="SmoothStepEdgeInternal";function mk(e){return C.memo(({id:t,...s})=>{const a=e.isInternal?void 0:t;return d.jsx(fk,{...s,id:a,pathOptions:C.useMemo(()=>({borderRadius:0,offset:s.pathOptions?.offset}),[s.pathOptions?.offset])})})}const DL=mk({isInternal:!1}),hk=mk({isInternal:!0});DL.displayName="StepEdge";hk.displayName="StepEdgeInternal";function gk(e){return C.memo(({id:t,sourceX:s,sourceY:a,targetX:o,targetY:l,label:c,labelStyle:f,labelShowBg:m,labelBgStyle:p,labelBgPadding:_,labelBgBorderRadius:y,style:b,markerEnd:v,markerStart:S,interactionWidth:w})=>{const[k,j,A]=z1({sourceX:s,sourceY:a,targetX:o,targetY:l}),N=e.isInternal?void 0:t;return d.jsx(rf,{id:N,path:k,labelX:j,labelY:A,label:c,labelStyle:f,labelShowBg:m,labelBgStyle:p,labelBgPadding:_,labelBgBorderRadius:y,style:b,markerEnd:v,markerStart:S,interactionWidth:w})})}const $L=gk({isInternal:!1}),yk=gk({isInternal:!0});$L.displayName="StraightEdge";yk.displayName="StraightEdgeInternal";function _k(e){return C.memo(({id:t,sourceX:s,sourceY:a,targetX:o,targetY:l,sourcePosition:c=ze.Bottom,targetPosition:f=ze.Top,label:m,labelStyle:p,labelShowBg:_,labelBgStyle:y,labelBgPadding:b,labelBgBorderRadius:v,style:S,markerEnd:w,markerStart:k,pathOptions:j,interactionWidth:A})=>{const[N,T,E]=O1({sourceX:s,sourceY:a,sourcePosition:c,targetX:o,targetY:l,targetPosition:f,curvature:j?.curvature}),O=e.isInternal?void 0:t;return d.jsx(rf,{id:O,path:N,labelX:T,labelY:E,label:m,labelStyle:p,labelShowBg:_,labelBgStyle:y,labelBgPadding:b,labelBgBorderRadius:v,style:S,markerEnd:w,markerStart:k,interactionWidth:A})})}const RL=_k({isInternal:!1}),bk=_k({isInternal:!0});RL.displayName="BezierEdge";bk.displayName="BezierEdgeInternal";const xx={default:bk,straight:yk,step:hk,smoothstep:pk,simplebezier:uk},wx={sourceX:null,sourceY:null,targetX:null,targetY:null,sourcePosition:null,targetPosition:null,zIndex:void 0},LL=(e,t,s)=>s===ze.Left?e-t:s===ze.Right?e+t:e,qL=(e,t,s)=>s===ze.Top?e-t:s===ze.Bottom?e+t:e,Sx="react-flow__edgeupdater";function kx({position:e,centerX:t,centerY:s,radius:a=10,onMouseDown:o,onMouseEnter:l,onMouseOut:c,type:f}){return d.jsx("circle",{onMouseDown:o,onMouseEnter:l,onMouseOut:c,className:an([Sx,`${Sx}-${f}`]),cx:LL(t,a,e),cy:qL(s,a,e),r:a,stroke:"transparent",fill:"transparent"})}function UL({isReconnectable:e,reconnectRadius:t,edge:s,sourceX:a,sourceY:o,targetX:l,targetY:c,sourcePosition:f,targetPosition:m,onReconnect:p,onReconnectStart:_,onReconnectEnd:y,setReconnecting:b,setUpdateHover:v}){const S=Et(),w=(T,E)=>{if(T.button!==0)return;const{autoPanOnConnect:O,domNode:R,connectionMode:q,connectionRadius:Z,lib:oe,onConnectStart:Q,cancelConnection:ee,nodeLookup:X,rfId:L,panBy:I,updateConnection:D}=S.getState(),U=E.type==="target",B=(z,Y)=>{b(!1),y?.(z,s,E.type,Y)},V=z=>p?.(s,z),te=(z,Y)=>{b(!0),_?.(T,s,E.type),Q?.(z,Y)};vh.onPointerDown(T.nativeEvent,{autoPanOnConnect:O,connectionMode:q,connectionRadius:Z,domNode:R,handleId:E.id,nodeId:E.nodeId,nodeLookup:X,isTarget:U,edgeUpdaterType:E.type,lib:oe,flowId:L,cancelConnection:ee,panBy:I,isValidConnection:(...z)=>S.getState().isValidConnection?.(...z)??!0,onConnect:V,onConnectStart:te,onConnectEnd:(...z)=>S.getState().onConnectEnd?.(...z),onReconnectEnd:B,updateConnection:D,getTransform:()=>S.getState().transform,getFromHandle:()=>S.getState().connection.fromHandle,dragThreshold:S.getState().connectionDragThreshold,handleDomNode:T.currentTarget})},k=T=>w(T,{nodeId:s.target,id:s.targetHandle??null,type:"target"}),j=T=>w(T,{nodeId:s.source,id:s.sourceHandle??null,type:"source"}),A=()=>v(!0),N=()=>v(!1);return d.jsxs(d.Fragment,{children:[(e===!0||e==="source")&&d.jsx(kx,{position:f,centerX:a,centerY:o,radius:t,onMouseDown:k,onMouseEnter:A,onMouseOut:N,type:"source"}),(e===!0||e==="target")&&d.jsx(kx,{position:m,centerX:l,centerY:c,radius:t,onMouseDown:j,onMouseEnter:A,onMouseOut:N,type:"target"})]})}function BL({id:e,edgesFocusable:t,edgesReconnectable:s,elementsSelectable:a,onClick:o,onDoubleClick:l,onContextMenu:c,onMouseEnter:f,onMouseMove:m,onMouseLeave:p,reconnectRadius:_,onReconnect:y,onReconnectStart:b,onReconnectEnd:v,rfId:S,edgeTypes:w,noPanClassName:k,onError:j,disableKeyboardA11y:A}){let N=it(we=>we.edgeLookup.get(e));const T=it(we=>we.defaultEdgeOptions);N=T?{...T,...N}:N;let E=N.type||"default",O=w?.[E]||xx[E];O===void 0&&(j?.("011",Ds.error011(E)),E="default",O=w?.default||xx.default);const R=!!(N.focusable||t&&typeof N.focusable>"u"),q=typeof y<"u"&&(N.reconnectable||s&&typeof N.reconnectable>"u"),Z=!!(N.selectable||a&&typeof N.selectable>"u"),oe=C.useRef(null),[Q,ee]=C.useState(!1),[X,L]=C.useState(!1),I=Et(),{zIndex:D=N.zIndex,sourceX:U,sourceY:B,targetX:V,targetY:te,sourcePosition:z,targetPosition:Y}=it(C.useCallback(we=>{const ve=we.nodeLookup.get(N.source),xe=we.nodeLookup.get(N.target);if(!ve||!xe)return wx;const ke=N3({id:e,sourceNode:ve,targetNode:xe,sourceHandle:N.sourceHandle||null,targetHandle:N.targetHandle||null,connectionMode:we.connectionMode,onError:j}),me=y3({selected:N.selected,zIndex:N.zIndex,sourceNode:ve,targetNode:xe,elevateOnSelect:we.elevateEdgesOnSelect,zIndexMode:we.zIndexMode});return{...ke||wx,zIndex:me}},[N.source,N.target,N.sourceHandle,N.targetHandle,N.selected,N.zIndex,j]),Lt),ce=C.useMemo(()=>N.markerStart?`url('#${_h(N.markerStart,S)}')`:void 0,[N.markerStart,S]),de=C.useMemo(()=>N.markerEnd?`url('#${_h(N.markerEnd,S)}')`:void 0,[N.markerEnd,S]);if(N.hidden||U===null||B===null||V===null||te===null)return null;const _e=we=>{const{addSelectedEdges:ve,unselectNodesAndEdges:xe,multiSelectionActive:ke}=I.getState();Z&&(I.setState({nodesSelectionActive:!1}),N.selected&&ke?(xe({nodes:[],edges:[N]}),oe.current?.blur()):ve([e])),o&&o(we,N)},F=l?we=>{l(we,{...N})}:void 0,G=c?we=>{c(we,{...N})}:void 0,P=f?we=>{f(we,{...N})}:void 0,ae=m?we=>{m(we,{...N})}:void 0,Se=p?we=>{p(we,{...N})}:void 0,je=we=>{if(!A&&h1.includes(we.key)&&Z){const{unselectNodesAndEdges:ve,addSelectedEdges:xe}=I.getState();we.key==="Escape"?(oe.current?.blur(),ve({edges:[N]})):xe([e])}};return d.jsx("svg",{style:{zIndex:D},children:d.jsxs("g",{className:an(["react-flow__edge",`react-flow__edge-${E}`,N.className,k,{selected:N.selected,animated:N.animated,inactive:!Z&&!o,updating:Q,selectable:Z}]),onClick:_e,onDoubleClick:F,onContextMenu:G,onMouseEnter:P,onMouseMove:ae,onMouseLeave:Se,onKeyDown:R?je:void 0,tabIndex:R?0:void 0,role:N.ariaRole??(R?"group":"img"),"aria-roledescription":"edge","data-id":e,"data-testid":`rf__edge-${e}`,"aria-label":N.ariaLabel===null?void 0:N.ariaLabel||`Edge from ${N.source} to ${N.target}`,"aria-describedby":R?`${G1}-${S}`:void 0,ref:oe,...N.domAttributes,children:[!X&&d.jsx(O,{id:e,source:N.source,target:N.target,type:N.type,selected:N.selected,animated:N.animated,selectable:Z,deletable:N.deletable??!0,label:N.label,labelStyle:N.labelStyle,labelShowBg:N.labelShowBg,labelBgStyle:N.labelBgStyle,labelBgPadding:N.labelBgPadding,labelBgBorderRadius:N.labelBgBorderRadius,sourceX:U,sourceY:B,targetX:V,targetY:te,sourcePosition:z,targetPosition:Y,data:N.data,style:N.style,sourceHandleId:N.sourceHandle,targetHandleId:N.targetHandle,markerStart:ce,markerEnd:de,pathOptions:"pathOptions"in N?N.pathOptions:void 0,interactionWidth:N.interactionWidth}),q&&d.jsx(UL,{edge:N,isReconnectable:q,reconnectRadius:_,onReconnect:y,onReconnectStart:b,onReconnectEnd:v,sourceX:U,sourceY:B,targetX:V,targetY:te,sourcePosition:z,targetPosition:Y,setUpdateHover:ee,setReconnecting:L})]})})}var HL=C.memo(BL);const IL=e=>({edgesFocusable:e.edgesFocusable,edgesReconnectable:e.edgesReconnectable,elementsSelectable:e.elementsSelectable,connectionMode:e.connectionMode,onError:e.onError});function vk({defaultMarkerColor:e,onlyRenderVisibleElements:t,rfId:s,edgeTypes:a,noPanClassName:o,onReconnect:l,onEdgeContextMenu:c,onEdgeMouseEnter:f,onEdgeMouseMove:m,onEdgeMouseLeave:p,onEdgeClick:_,reconnectRadius:y,onEdgeDoubleClick:b,onReconnectStart:v,onReconnectEnd:S,disableKeyboardA11y:w}){const{edgesFocusable:k,edgesReconnectable:j,elementsSelectable:A,onError:N}=it(IL,Lt),T=NL(t);return d.jsxs("div",{className:"react-flow__edges",children:[d.jsx(OL,{defaultColor:e,rfId:s}),T.map(E=>d.jsx(HL,{id:E,edgesFocusable:k,edgesReconnectable:j,elementsSelectable:A,noPanClassName:o,onReconnect:l,onContextMenu:c,onMouseEnter:f,onMouseMove:m,onMouseLeave:p,onClick:_,reconnectRadius:y,onDoubleClick:b,onReconnectStart:v,onReconnectEnd:S,rfId:s,onError:N,edgeTypes:a,disableKeyboardA11y:w},E))]})}vk.displayName="EdgeRenderer";const ZL=C.memo(vk),jx=e=>`translate(${e[0]}px,${e[1]}px) scale(${e[2]})`;function PL({children:e}){const t=Et(),s=C.useRef(null),[a]=C.useState(()=>t.getState().transform);return Q1(()=>{let o=null;const l=()=>{const c=t.getState().transform;o&&c[0]===o[0]&&c[1]===o[1]&&c[2]===o[2]||(o=c,s.current&&(s.current.style.transform=jx(c)))};return l(),t.subscribe(l)},[t]),d.jsx("div",{ref:s,className:"react-flow__viewport xyflow__viewport react-flow__container",style:{transform:jx(a)},children:e})}function YL(e){const t=Sg(),s=C.useRef(!1);C.useEffect(()=>{!s.current&&t.viewportInitialized&&e&&(setTimeout(()=>e(t),1),s.current=!0)},[e,t.viewportInitialized])}const VL=e=>e.panZoom?.syncViewport;function KL(e){const t=it(VL),s=Et();return C.useEffect(()=>{e&&(t?.(e),s.setState({transform:[e.x,e.y,e.zoom]}))},[e,t]),null}function GL(e){return e.connection.inProgress?{...e.connection,to:fc(e.connection.to,e.transform)}:{...e.connection}}function XL(e){return GL}function FL(e){const t=XL();return it(t,Lt)}const JL=e=>({nodesConnectable:e.nodesConnectable,isValid:e.connection.isValid,inProgress:e.connection.inProgress,width:e.width,height:e.height});function QL({containerStyle:e,style:t,type:s,component:a}){const{nodesConnectable:o,width:l,height:c,isValid:f,inProgress:m}=it(JL,Lt);return!(l&&o&&m)?null:d.jsx("svg",{style:e,width:l,height:c,className:"react-flow__connectionline react-flow__container",children:d.jsx("g",{className:an(["react-flow__connection",N1(f)]),children:d.jsx(xk,{style:t,type:s,CustomComponent:a,isValid:f})})})}const xk=({style:e,type:t=pa.Bezier,CustomComponent:s,isValid:a})=>{const{inProgress:o,from:l,fromNode:c,fromHandle:f,fromPosition:m,to:p,toNode:_,toHandle:y,toPosition:b,pointer:v}=FL();if(!o)return;if(s)return d.jsx(s,{connectionLineType:t,connectionLineStyle:e,fromNode:c,fromHandle:f,fromX:l.x,fromY:l.y,toX:p.x,toY:p.y,fromPosition:m,toPosition:b,connectionStatus:N1(a),toNode:_,toHandle:y,pointer:v});let S="";const w={sourceX:l.x,sourceY:l.y,sourcePosition:m,targetX:p.x,targetY:p.y,targetPosition:b};switch(t){case pa.Bezier:[S]=O1(w);break;case pa.SimpleBezier:[S]=lk(w);break;case pa.Step:[S]=yh({...w,borderRadius:0});break;case pa.SmoothStep:[S]=yh(w);break;default:[S]=z1(w)}return d.jsx("path",{d:S,fill:"none",className:"react-flow__connection-path",style:e})};xk.displayName="ConnectionLine";const WL={};function Nx(e=WL){C.useRef(e),Et(),C.useEffect(()=>{},[e])}function e6(){Et(),C.useRef(!1),C.useEffect(()=>{},[])}function wk({nodeTypes:e,edgeTypes:t,onInit:s,onNodeClick:a,onEdgeClick:o,onNodeDoubleClick:l,onEdgeDoubleClick:c,onNodeMouseEnter:f,onNodeMouseMove:m,onNodeMouseLeave:p,onNodeContextMenu:_,onSelectionContextMenu:y,onSelectionStart:b,onSelectionEnd:v,connectionLineType:S,connectionLineStyle:w,connectionLineComponent:k,connectionLineContainerStyle:j,selectionKeyCode:A,selectionOnDrag:N,selectionMode:T,multiSelectionKeyCode:E,panActivationKeyCode:O,zoomActivationKeyCode:R,deleteKeyCode:q,onlyRenderVisibleElements:Z,elementsSelectable:oe,defaultViewport:Q,translateExtent:ee,minZoom:X,maxZoom:L,preventScrolling:I,defaultMarkerColor:D,zoomOnScroll:U,zoomOnPinch:B,panOnScroll:V,panOnScrollSpeed:te,panOnScrollMode:z,zoomOnDoubleClick:Y,panOnDrag:ce,autoPanOnSelection:de,onPaneClick:_e,onPaneMouseEnter:F,onPaneMouseMove:G,onPaneMouseLeave:P,onPaneScroll:ae,onPaneContextMenu:Se,paneClickDistance:je,nodeClickDistance:we,onEdgeContextMenu:ve,onEdgeMouseEnter:xe,onEdgeMouseMove:ke,onEdgeMouseLeave:me,reconnectRadius:Ge,onReconnect:Ke,onReconnectStart:rt,onReconnectEnd:nt,noDragClassName:qe,noWheelClassName:Xt,noPanClassName:Pt,disableKeyboardA11y:lt,nodeExtent:St,rfId:mn,viewport:Tt,onViewportChange:Ft,nodesDraggable:Jt}){return Nx(e),Nx(t),e6(),YL(s),KL(Tt),d.jsx(gL,{onPaneClick:_e,onPaneMouseEnter:F,onPaneMouseMove:G,onPaneMouseLeave:P,onPaneContextMenu:Se,onPaneScroll:ae,paneClickDistance:je,deleteKeyCode:q,selectionKeyCode:A,selectionOnDrag:N,selectionMode:T,onSelectionStart:b,onSelectionEnd:v,multiSelectionKeyCode:E,panActivationKeyCode:O,zoomActivationKeyCode:R,elementsSelectable:oe,zoomOnScroll:U,zoomOnPinch:B,zoomOnDoubleClick:Y,panOnScroll:V,panOnScrollSpeed:te,panOnScrollMode:z,panOnDrag:ce,autoPanOnSelection:de,defaultViewport:Q,translateExtent:ee,minZoom:X,maxZoom:L,onSelectionContextMenu:y,preventScrolling:I,noDragClassName:qe,noWheelClassName:Xt,noPanClassName:Pt,disableKeyboardA11y:lt,onViewportChange:Ft,isControlledViewport:!!Tt,children:d.jsxs(PL,{children:[d.jsx(ZL,{edgeTypes:t,onEdgeClick:o,onEdgeDoubleClick:c,onReconnect:Ke,onReconnectStart:rt,onReconnectEnd:nt,onlyRenderVisibleElements:Z,onEdgeContextMenu:ve,onEdgeMouseEnter:xe,onEdgeMouseMove:ke,onEdgeMouseLeave:me,reconnectRadius:Ge,defaultMarkerColor:D,noPanClassName:Pt,disableKeyboardA11y:lt,rfId:mn}),d.jsx(QL,{style:w,type:S,component:k,containerStyle:j}),d.jsx("div",{className:"react-flow__edgelabel-renderer"}),d.jsx(jL,{nodeTypes:e,onNodeClick:a,onNodeDoubleClick:l,onNodeMouseEnter:f,onNodeMouseMove:m,onNodeMouseLeave:p,onNodeContextMenu:_,nodeClickDistance:we,onlyRenderVisibleElements:Z,noPanClassName:Pt,noDragClassName:qe,disableKeyboardA11y:lt,nodeExtent:St,rfId:mn,nodesDraggable:Jt}),d.jsx("div",{className:"react-flow__viewport-portal"})]})})}wk.displayName="GraphView";const t6=C.memo(wk),n6=S1(),Ex=({nodes:e,edges:t,defaultNodes:s,defaultEdges:a,width:o,height:l,fitView:c,fitViewOptions:f,minZoom:m=.5,maxZoom:p=2,nodeOrigin:_,nodeExtent:y,zIndexMode:b="basic"}={})=>{const v=new Map,S=new Map,w=new Map,k=new Map,j=a??t??[],A=s??e??[],N=_??[0,0],T=y??Kl;R1(w,k,j);const{nodesInitialized:E}=bh(A,v,S,{nodeOrigin:N,nodeExtent:T,zIndexMode:b});let O=[0,0,1];if(c&&o&&l){const R=uc(v,{filter:Q=>!!((Q.width||Q.initialWidth)&&(Q.height||Q.initialHeight))}),{x:q,y:Z,zoom:oe}=gg(R,o,l,m,p,f?.padding??.1);O=[q,Z,oe]}return{rfId:"1",width:o??0,height:l??0,transform:O,nodes:A,nodesInitialized:E,nodeLookup:v,parentLookup:S,edges:j,edgeLookup:k,connectionLookup:w,onNodesChange:null,onEdgesChange:null,hasDefaultNodes:s!==void 0,hasDefaultEdges:a!==void 0,panZoom:null,minZoom:m,maxZoom:p,translateExtent:Kl,nodeExtent:T,nodesSelectionActive:!1,userSelectionActive:!1,userSelectionRect:null,connectionMode:jr.Strict,domNode:null,paneDragging:!1,noPanClassName:"nopan",nodeOrigin:N,nodeDragThreshold:1,connectionDragThreshold:1,snapGrid:[15,15],snapToGrid:!1,nodesDraggable:!0,nodesConnectable:!0,nodesFocusable:!0,edgesFocusable:!0,edgesReconnectable:!0,elementsSelectable:!0,elevateNodesOnSelect:!0,elevateEdgesOnSelect:!0,selectNodesOnDrag:!0,multiSelectionActive:!1,fitViewQueued:c??!1,fitViewOptions:f,fitViewResolver:null,connection:{...y1},connectionClickStartHandle:null,connectOnClick:!0,ariaLiveMessage:"",autoPanOnConnect:!0,autoPanOnNodeDrag:!0,autoPanOnNodeFocus:!0,autoPanSpeed:15,connectionRadius:20,onError:n6,isValidConnection:void 0,onSelectionChangeHandlers:[],lib:"react",debug:!1,ariaLabelConfig:g1,zIndexMode:b,defaultEdgeOptions:void 0,onNodesChangeMiddlewareMap:new Map,onEdgesChangeMiddlewareMap:new Map,onNodesDelete:void 0,onEdgesDelete:void 0,onDelete:void 0,onBeforeDelete:void 0,onViewportChangeStart:void 0,onViewportChange:void 0,onViewportChangeEnd:void 0,onNodeDragStart:void 0,onNodeDrag:void 0,onNodeDragStop:void 0,onSelectionDragStart:void 0,onSelectionDrag:void 0,onSelectionDragStop:void 0,onMoveStart:void 0,onMove:void 0,onMoveEnd:void 0,onConnect:void 0,onConnectStart:void 0,onConnectEnd:void 0,onClickConnectStart:void 0,onClickConnectEnd:void 0}},s6=({nodes:e,edges:t,defaultNodes:s,defaultEdges:a,width:o,height:l,fitView:c,fitViewOptions:f,minZoom:m,maxZoom:p,nodeOrigin:_,nodeExtent:y,zIndexMode:b})=>h4((v,S)=>{async function w(){const{nodeLookup:k,panZoom:j,fitViewOptions:A,fitViewResolver:N,width:T,height:E,minZoom:O,maxZoom:R}=S();j&&(await u3({nodes:k,width:T,height:E,panZoom:j,minZoom:O,maxZoom:R},A),N?.resolve(!0),v({fitViewResolver:null}))}return{...Ex({nodes:e,edges:t,width:o,height:l,fitView:c,fitViewOptions:f,minZoom:m,maxZoom:p,nodeOrigin:_,nodeExtent:y,defaultNodes:s,defaultEdges:a,zIndexMode:b}),setNodes:k=>{const{nodeLookup:j,parentLookup:A,nodeOrigin:N,nodeExtent:T,elevateNodesOnSelect:E,fitViewQueued:O,zIndexMode:R,nodesSelectionActive:q}=S(),{nodesInitialized:Z,hasSelectedNodes:oe}=bh(k,j,A,{nodeOrigin:N,nodeExtent:T,elevateNodesOnSelect:E,checkEquality:!0,zIndexMode:R}),Q=q&&oe;O&&Z?(w(),v({nodes:k,nodesInitialized:Z,fitViewQueued:!1,fitViewOptions:void 0,nodesSelectionActive:Q})):v({nodes:k,nodesInitialized:Z,nodesSelectionActive:Q})},setEdges:k=>{const{connectionLookup:j,edgeLookup:A}=S();R1(j,A,k),v({edges:k})},setDefaultNodesAndEdges:(k,j)=>{if(k){const{setNodes:A}=S();A(k),v({hasDefaultNodes:!0})}if(j){const{setEdges:A}=S();A(j),v({hasDefaultEdges:!0})}},updateNodeInternals:k=>{const{triggerNodeChanges:j,nodeLookup:A,parentLookup:N,domNode:T,nodeOrigin:E,nodeExtent:O,debug:R,fitViewQueued:q,zIndexMode:Z}=S(),{changes:oe,updatedInternals:Q}=D3(k,A,N,T,E,O,Z);Q&&(A3(A,N,{nodeOrigin:E,nodeExtent:O,zIndexMode:Z}),q?(w(),v({fitViewQueued:!1,fitViewOptions:void 0})):v({}),oe?.length>0&&(R&&console.log("React Flow: trigger node changes",oe),j?.(oe)))},updateNodePositions:(k,j=!1)=>{const A=[];let N=[];const{nodeLookup:T,triggerNodeChanges:E,connection:O,updateConnection:R,onNodesChangeMiddlewareMap:q}=S();for(const[Z,oe]of k){const Q=T.get(Z),ee=!!(Q?.expandParent&&Q?.parentId&&oe?.position),X={id:Z,type:"position",position:ee?{x:Math.max(0,oe.position.x),y:Math.max(0,oe.position.y)}:oe.position,dragging:j};if(Q&&O.inProgress&&O.fromNode.id===Q.id){const L=mo(Q,O.fromHandle,ze.Left,!0);R({...O,from:L})}ee&&Q.parentId&&A.push({id:Z,parentId:Q.parentId,rect:{...oe.internals.positionAbsolute,width:oe.measured.width??0,height:oe.measured.height??0}}),N.push(X)}if(A.length>0){const{parentLookup:Z,nodeOrigin:oe}=S(),Q=wg(A,T,Z,oe);N.push(...Q)}for(const Z of q.values())N=Z(N);E(N)},triggerNodeChanges:k=>{const{onNodesChange:j,setNodes:A,nodes:N,hasDefaultNodes:T,debug:E}=S();if(k?.length){if(T){const O=R4(k,N);A(O)}E&&console.log("React Flow: trigger node changes",k),j?.(k)}},triggerEdgeChanges:k=>{const{onEdgesChange:j,setEdges:A,edges:N,hasDefaultEdges:T,debug:E}=S();if(k?.length){if(T){const O=L4(k,N);A(O)}E&&console.log("React Flow: trigger edge changes",k),j?.(k)}},addSelectedNodes:k=>{const{multiSelectionActive:j,edgeLookup:A,nodeLookup:N,triggerNodeChanges:T,triggerEdgeChanges:E}=S();if(j){const O=k.map(R=>Ga(R,!0));T(O);return}T(cr(N,new Set([...k]))),E(cr(A))},addSelectedEdges:k=>{const{multiSelectionActive:j,edgeLookup:A,nodeLookup:N,triggerNodeChanges:T,triggerEdgeChanges:E}=S();if(j){const O=k.map(R=>Ga(R,!0));E(O);return}E(cr(A,new Set([...k]))),T(cr(N,new Set))},unselectNodesAndEdges:({nodes:k,edges:j}={})=>{const{edges:A,nodes:N,nodeLookup:T,triggerNodeChanges:E,triggerEdgeChanges:O}=S(),R=k||N,q=j||A,Z=[];for(const Q of R)Q.selected&&Z.push(Ga(Q.id,!1));const oe=[];for(const Q of q)Q.selected&&oe.push(Ga(Q.id,!1));E(Z),O(oe)},setMinZoom:k=>{const{panZoom:j,maxZoom:A}=S();j?.setScaleExtent([k,A]),v({minZoom:k})},setMaxZoom:k=>{const{panZoom:j,minZoom:A}=S();j?.setScaleExtent([A,k]),v({maxZoom:k})},setTranslateExtent:k=>{S().panZoom?.setTranslateExtent(k),v({translateExtent:k})},resetSelectedElements:()=>{const{edges:k,nodes:j,triggerNodeChanges:A,triggerEdgeChanges:N,elementsSelectable:T}=S();if(!T)return;const E=j.reduce((R,q)=>q.selected?[...R,Ga(q.id,!1)]:R,[]),O=k.reduce((R,q)=>q.selected?[...R,Ga(q.id,!1)]:R,[]);A(E),N(O)},setNodeExtent:k=>{const{nodes:j,nodeLookup:A,parentLookup:N,nodeOrigin:T,elevateNodesOnSelect:E,nodeExtent:O,zIndexMode:R}=S();k[0][0]===O[0][0]&&k[0][1]===O[0][1]&&k[1][0]===O[1][0]&&k[1][1]===O[1][1]||(bh(j,A,N,{nodeOrigin:T,nodeExtent:k,elevateNodesOnSelect:E,checkEquality:!1,zIndexMode:R}),v({nodeExtent:k}))},panBy:k=>{const{transform:j,width:A,height:N,panZoom:T,translateExtent:E}=S();return $3({delta:k,panZoom:T,transform:j,translateExtent:E,width:A,height:N})},setCenter:async(k,j,A)=>{const{width:N,height:T,maxZoom:E,panZoom:O}=S();if(!O)return!1;const R=typeof A?.zoom<"u"?A.zoom:E;return await O.setViewport({x:N/2-k*R,y:T/2-j*R,zoom:R},{duration:A?.duration,ease:A?.ease,interpolate:A?.interpolate}),!0},cancelConnection:()=>{v({connection:{...y1}})},updateConnection:k=>{v({connection:k})},reset:()=>v({...Ex()})}},Object.is);function i6({initialNodes:e,initialEdges:t,defaultNodes:s,defaultEdges:a,initialWidth:o,initialHeight:l,initialMinZoom:c,initialMaxZoom:f,initialFitViewOptions:m,fitView:p,nodeOrigin:_,nodeExtent:y,zIndexMode:b,children:v}){const[S]=C.useState(()=>s6({nodes:e,edges:t,defaultNodes:s,defaultEdges:a,width:o,height:l,fitView:p,minZoom:c,maxZoom:f,fitViewOptions:m,nodeOrigin:_,nodeExtent:y,zIndexMode:b}));return d.jsx(g4,{value:S,children:d.jsx(I4,{children:d.jsx(sL,{children:v})})})}function a6({children:e,nodes:t,edges:s,defaultNodes:a,defaultEdges:o,width:l,height:c,fitView:f,fitViewOptions:m,minZoom:p,maxZoom:_,nodeOrigin:y,nodeExtent:b,zIndexMode:v}){return C.useContext(af)?d.jsx(d.Fragment,{children:e}):d.jsx(i6,{initialNodes:t,initialEdges:s,defaultNodes:a,defaultEdges:o,initialWidth:l,initialHeight:c,fitView:f,initialFitViewOptions:m,initialMinZoom:p,initialMaxZoom:_,nodeOrigin:y,nodeExtent:b,zIndexMode:v,children:e})}const o6={width:"100%",height:"100%",overflow:"hidden",position:"relative",zIndex:0};function r6({nodes:e,edges:t,defaultNodes:s,defaultEdges:a,className:o,nodeTypes:l,edgeTypes:c,onNodeClick:f,onEdgeClick:m,onInit:p,onMove:_,onMoveStart:y,onMoveEnd:b,onConnect:v,onConnectStart:S,onConnectEnd:w,onClickConnectStart:k,onClickConnectEnd:j,onNodeMouseEnter:A,onNodeMouseMove:N,onNodeMouseLeave:T,onNodeContextMenu:E,onNodeDoubleClick:O,onNodeDragStart:R,onNodeDrag:q,onNodeDragStop:Z,onNodesDelete:oe,onEdgesDelete:Q,onDelete:ee,onSelectionChange:X,onSelectionDragStart:L,onSelectionDrag:I,onSelectionDragStop:D,onSelectionContextMenu:U,onSelectionStart:B,onSelectionEnd:V,onBeforeDelete:te,connectionMode:z,connectionLineType:Y=pa.Bezier,connectionLineStyle:ce,connectionLineComponent:de,connectionLineContainerStyle:_e,deleteKeyCode:F="Backspace",selectionKeyCode:G="Shift",selectionOnDrag:P=!1,selectionMode:ae=Gl.Full,panActivationKeyCode:Se="Space",multiSelectionKeyCode:je=Jl()?"Meta":"Control",zoomActivationKeyCode:we=Jl()?"Meta":"Control",snapToGrid:ve,snapGrid:xe,onlyRenderVisibleElements:ke=!1,selectNodesOnDrag:me,nodesDraggable:Ge,autoPanOnNodeFocus:Ke,nodesConnectable:rt,nodesFocusable:nt,nodeOrigin:qe=X1,edgesFocusable:Xt,edgesReconnectable:Pt,elementsSelectable:lt=!0,defaultViewport:St=C4,minZoom:mn=.5,maxZoom:Tt=2,translateExtent:Ft=Kl,preventScrolling:Jt=!0,nodeExtent:zn,defaultMarkerColor:zi="#b1b1b7",zoomOnScroll:Us=!0,zoomOnPinch:Dn=!0,panOnScroll:Qt=!1,panOnScrollSpeed:Be=.5,panOnScrollMode:Mt=so.Free,zoomOnDoubleClick:Bs=!0,panOnDrag:va=!0,onPaneClick:le,onPaneMouseEnter:Te,onPaneMouseMove:Re,onPaneMouseLeave:Ne,onPaneScroll:Ce,onPaneContextMenu:ct,paneClickDistance:wt=1,nodeClickDistance:Vn=0,children:Ws,onReconnect:vs,onReconnectStart:Nn,onReconnectEnd:Hs,onEdgeContextMenu:ei,onEdgeDoubleClick:xa,onEdgeMouseEnter:ti,onEdgeMouseMove:Di,onEdgeMouseLeave:ni,reconnectRadius:wa=10,onNodesChange:Sa,onEdgesChange:En,noDragClassName:kt="nodrag",noWheelClassName:Yt="nowheel",noPanClassName:as="nopan",fitView:xs,fitViewOptions:$i,connectOnClick:Je,attributionPosition:hn,proOptions:gn,defaultEdgeOptions:yn,elevateNodesOnSelect:un=!0,elevateEdgesOnSelect:Tn=!1,disableKeyboardA11y:zt=!1,autoPanOnConnect:Cn,autoPanOnNodeDrag:Dt,autoPanOnSelection:bo=!0,autoPanSpeed:ws,connectionRadius:os,isValidConnection:rs,onError:vo,style:ka,id:ja,nodeDragThreshold:xo,connectionDragThreshold:Ss,viewport:si,onViewportChange:$n,width:wn,height:Ri,colorMode:Na="light",debug:Ea,onScroll:Ta,ariaLabelConfig:Li,zIndexMode:wo="basic",...Sn},ii){const qi=ja||"1",Ca=z4(Na),Is=C.useCallback(Ui=>{Ui.currentTarget.scrollTo({top:0,left:0,behavior:"instant"}),Ta?.(Ui)},[Ta]);return d.jsx("div",{"data-testid":"rf__wrapper",...Sn,onScroll:Is,style:{...ka,...o6},ref:ii,className:an(["react-flow",o,Ca]),id:ja,role:"application",children:d.jsxs(a6,{nodes:e,edges:t,width:wn,height:Ri,fitView:xs,fitViewOptions:$i,minZoom:mn,maxZoom:Tt,nodeOrigin:qe,nodeExtent:zn,zIndexMode:wo,children:[d.jsx(M4,{nodes:e,edges:t,defaultNodes:s,defaultEdges:a,onConnect:v,onConnectStart:S,onConnectEnd:w,onClickConnectStart:k,onClickConnectEnd:j,nodesDraggable:Ge,autoPanOnNodeFocus:Ke,nodesConnectable:rt,nodesFocusable:nt,edgesFocusable:Xt,edgesReconnectable:Pt,elementsSelectable:lt,elevateNodesOnSelect:un,elevateEdgesOnSelect:Tn,minZoom:mn,maxZoom:Tt,nodeExtent:zn,onNodesChange:Sa,onEdgesChange:En,snapToGrid:ve,snapGrid:xe,connectionMode:z,translateExtent:Ft,connectOnClick:Je,defaultEdgeOptions:yn,fitView:xs,fitViewOptions:$i,onNodesDelete:oe,onEdgesDelete:Q,onDelete:ee,onNodeDragStart:R,onNodeDrag:q,onNodeDragStop:Z,onSelectionDrag:I,onSelectionDragStart:L,onSelectionDragStop:D,onMove:_,onMoveStart:y,onMoveEnd:b,noPanClassName:as,nodeOrigin:qe,rfId:qi,autoPanOnConnect:Cn,autoPanOnNodeDrag:Dt,autoPanSpeed:ws,onError:vo,connectionRadius:os,isValidConnection:rs,selectNodesOnDrag:me,nodeDragThreshold:xo,connectionDragThreshold:Ss,onBeforeDelete:te,debug:Ea,ariaLabelConfig:Li,zIndexMode:wo}),d.jsx(t6,{onInit:p,onNodeClick:f,onEdgeClick:m,onNodeMouseEnter:A,onNodeMouseMove:N,onNodeMouseLeave:T,onNodeContextMenu:E,onNodeDoubleClick:O,nodeTypes:l,edgeTypes:c,connectionLineType:Y,connectionLineStyle:ce,connectionLineComponent:de,connectionLineContainerStyle:_e,selectionKeyCode:G,selectionOnDrag:P,selectionMode:ae,deleteKeyCode:F,multiSelectionKeyCode:je,panActivationKeyCode:Se,zoomActivationKeyCode:we,onlyRenderVisibleElements:ke,defaultViewport:St,translateExtent:Ft,minZoom:mn,maxZoom:Tt,preventScrolling:Jt,zoomOnScroll:Us,zoomOnPinch:Dn,zoomOnDoubleClick:Bs,panOnScroll:Qt,panOnScrollSpeed:Be,panOnScrollMode:Mt,panOnDrag:va,autoPanOnSelection:bo,onPaneClick:le,onPaneMouseEnter:Te,onPaneMouseMove:Re,onPaneMouseLeave:Ne,onPaneScroll:Ce,onPaneContextMenu:ct,paneClickDistance:wt,nodeClickDistance:Vn,onSelectionContextMenu:U,onSelectionStart:B,onSelectionEnd:V,onReconnect:vs,onReconnectStart:Nn,onReconnectEnd:Hs,onEdgeContextMenu:ei,onEdgeDoubleClick:xa,onEdgeMouseEnter:ti,onEdgeMouseMove:Di,onEdgeMouseLeave:ni,reconnectRadius:wa,defaultMarkerColor:zi,noDragClassName:kt,noWheelClassName:Yt,noPanClassName:as,rfId:qi,disableKeyboardA11y:zt,nodeExtent:zn,viewport:si,onViewportChange:$n,nodesDraggable:Ge}),d.jsx(T4,{onSelectionChange:X}),Ws,d.jsx(S4,{proOptions:gn,position:hn}),d.jsx(w4,{rfId:qi,disableKeyboardA11y:zt})]})})}var l6=J1(r6);function c6({dimensions:e,lineWidth:t,variant:s,className:a}){return d.jsx("path",{strokeWidth:t,d:`M${e[0]/2} 0 V${e[1]} M0 ${e[1]/2} H${e[0]}`,className:an(["react-flow__background-pattern",s,a])})}function u6({radius:e,className:t}){return d.jsx("circle",{cx:e,cy:e,r:e,className:an(["react-flow__background-pattern","dots",t])})}var Ci;(function(e){e.Lines="lines",e.Dots="dots",e.Cross="cross"})(Ci||(Ci={}));const d6={[Ci.Dots]:1,[Ci.Lines]:1,[Ci.Cross]:6},f6=e=>({transform:e.transform,patternId:`pattern-${e.rfId}`});function Sk({id:e,variant:t=Ci.Dots,gap:s=20,size:a,lineWidth:o=1,offset:l=0,color:c,bgColor:f,style:m,className:p,patternClassName:_}){const y=C.useRef(null),{transform:b,patternId:v}=it(f6,Lt),S=a||d6[t],w=t===Ci.Dots,k=t===Ci.Cross,j=Array.isArray(s)?s:[s,s],A=[j[0]*b[2]||1,j[1]*b[2]||1],N=S*b[2],T=Array.isArray(l)?l:[l,l],E=k?[N,N]:A,O=[T[0]*b[2]+E[0]/2,T[1]*b[2]+E[1]/2],R=`${v}${e||""}`;return d.jsxs("svg",{className:an(["react-flow__background",p]),style:{...m,...of,"--xy-background-color-props":f,"--xy-background-pattern-color-props":c},ref:y,"data-testid":"rf__background",children:[d.jsx("pattern",{id:R,x:b[0]%A[0],y:b[1]%A[1],width:A[0],height:A[1],patternUnits:"userSpaceOnUse",patternTransform:`translate(-${O[0]},-${O[1]})`,children:w?d.jsx(u6,{radius:N/2,className:_}):d.jsx(c6,{dimensions:E,lineWidth:o,variant:t,className:_})}),d.jsx("rect",{x:"0",y:"0",width:"100%",height:"100%",fill:`url(#${R})`})]})}Sk.displayName="Background";const p6=C.memo(Sk);function m6(){return d.jsx("svg",{xmlns:"http://www.w3.org/2000/svg",viewBox:"0 0 32 32",children:d.jsx("path",{d:"M32 18.133H18.133V32h-4.266V18.133H0v-4.266h13.867V0h4.266v13.867H32z"})})}function h6(){return d.jsx("svg",{xmlns:"http://www.w3.org/2000/svg",viewBox:"0 0 32 5",children:d.jsx("path",{d:"M0 0h32v4.2H0z"})})}function g6(){return d.jsx("svg",{xmlns:"http://www.w3.org/2000/svg",viewBox:"0 0 32 30",children:d.jsx("path",{d:"M3.692 4.63c0-.53.4-.938.939-.938h5.215V0H4.708C2.13 0 0 2.054 0 4.63v5.216h3.692V4.631zM27.354 0h-5.2v3.692h5.17c.53 0 .984.4.984.939v5.215H32V4.631A4.624 4.624 0 0027.354 0zm.954 24.83c0 .532-.4.94-.939.94h-5.215v3.768h5.215c2.577 0 4.631-2.13 4.631-4.707v-5.139h-3.692v5.139zm-23.677.94c-.531 0-.939-.4-.939-.94v-5.138H0v5.139c0 2.577 2.13 4.707 4.708 4.707h5.138V25.77H4.631z"})})}function y6(){return d.jsx("svg",{xmlns:"http://www.w3.org/2000/svg",viewBox:"0 0 25 32",children:d.jsx("path",{d:"M21.333 10.667H19.81V7.619C19.81 3.429 16.38 0 12.19 0 8 0 4.571 3.429 4.571 7.619v3.048H3.048A3.056 3.056 0 000 13.714v15.238A3.056 3.056 0 003.048 32h18.285a3.056 3.056 0 003.048-3.048V13.714a3.056 3.056 0 00-3.048-3.047zM12.19 24.533a3.056 3.056 0 01-3.047-3.047 3.056 3.056 0 013.047-3.048 3.056 3.056 0 013.048 3.048 3.056 3.056 0 01-3.048 3.047zm4.724-13.866H7.467V7.619c0-2.59 2.133-4.724 4.723-4.724 2.591 0 4.724 2.133 4.724 4.724v3.048z"})})}function _6(){return d.jsx("svg",{xmlns:"http://www.w3.org/2000/svg",viewBox:"0 0 25 32",children:d.jsx("path",{d:"M21.333 10.667H19.81V7.619C19.81 3.429 16.38 0 12.19 0c-4.114 1.828-1.37 2.133.305 2.438 1.676.305 4.42 2.59 4.42 5.181v3.048H3.047A3.056 3.056 0 000 13.714v15.238A3.056 3.056 0 003.048 32h18.285a3.056 3.056 0 003.048-3.048V13.714a3.056 3.056 0 00-3.048-3.047zM12.19 24.533a3.056 3.056 0 01-3.047-3.047 3.056 3.056 0 013.047-3.048 3.056 3.056 0 013.048 3.048 3.056 3.056 0 01-3.048 3.047z"})})}function Iu({children:e,className:t,...s}){return d.jsx("button",{type:"button",className:an(["react-flow__controls-button",t]),...s,children:e})}const b6=e=>({isInteractive:e.nodesDraggable||e.nodesConnectable||e.elementsSelectable,minZoomReached:e.transform[2]<=e.minZoom,maxZoomReached:e.transform[2]>=e.maxZoom,ariaLabelConfig:e.ariaLabelConfig});function kk({style:e,showZoom:t=!0,showFitView:s=!0,showInteractive:a=!0,fitViewOptions:o,onZoomIn:l,onZoomOut:c,onFitView:f,onInteractiveChange:m,className:p,children:_,position:y="bottom-left",orientation:b="vertical","aria-label":v}){const S=Et(),{isInteractive:w,minZoomReached:k,maxZoomReached:j,ariaLabelConfig:A}=it(b6,Lt),{zoomIn:N,zoomOut:T,fitView:E}=Sg(),O=()=>{N(),l?.()},R=()=>{T(),c?.()},q=()=>{E(o),f?.()},Z=()=>{S.setState({nodesDraggable:!w,nodesConnectable:!w,elementsSelectable:!w}),m?.(!w)},oe=b==="horizontal"?"horizontal":"vertical";return d.jsxs(Cr,{className:an(["react-flow__controls",oe,p]),position:y,style:e,"data-testid":"rf__controls","aria-label":v??A["controls.ariaLabel"],children:[t&&d.jsxs(d.Fragment,{children:[d.jsx(Iu,{onClick:O,className:"react-flow__controls-zoomin",title:A["controls.zoomIn.ariaLabel"],"aria-label":A["controls.zoomIn.ariaLabel"],disabled:j,children:d.jsx(m6,{})}),d.jsx(Iu,{onClick:R,className:"react-flow__controls-zoomout",title:A["controls.zoomOut.ariaLabel"],"aria-label":A["controls.zoomOut.ariaLabel"],disabled:k,children:d.jsx(h6,{})})]}),s&&d.jsx(Iu,{className:"react-flow__controls-fitview",onClick:q,title:A["controls.fitView.ariaLabel"],"aria-label":A["controls.fitView.ariaLabel"],children:d.jsx(g6,{})}),a&&d.jsx(Iu,{className:"react-flow__controls-interactive",onClick:Z,title:A["controls.interactive.ariaLabel"],"aria-label":A["controls.interactive.ariaLabel"],children:w?d.jsx(_6,{}):d.jsx(y6,{})}),_]})}kk.displayName="Controls";const v6=C.memo(kk);function x6({id:e,x:t,y:s,width:a,height:o,style:l,color:c,strokeColor:f,strokeWidth:m,className:p,borderRadius:_,shapeRendering:y,selected:b,onClick:v}){const{background:S,backgroundColor:w}=l||{},k=c||S||w;return d.jsx("rect",{className:an(["react-flow__minimap-node",{selected:b},p]),x:t,y:s,rx:_,ry:_,width:a,height:o,style:{fill:k,stroke:f,strokeWidth:m},shapeRendering:y,onClick:v?j=>v(j,e):void 0})}const w6=C.memo(x6),S6=e=>e.nodes.map(t=>t.id),Ym=e=>e instanceof Function?e:()=>e;function k6({nodeStrokeColor:e,nodeColor:t,nodeClassName:s="",nodeBorderRadius:a=5,nodeStrokeWidth:o,nodeComponent:l=w6,onClick:c}){const f=it(S6,Lt),m=Ym(t),p=Ym(e),_=Ym(s),y=typeof window>"u"||window.chrome?"crispEdges":"geometricPrecision";return d.jsx(d.Fragment,{children:f.map(b=>d.jsx(N6,{id:b,nodeColorFunc:m,nodeStrokeColorFunc:p,nodeClassNameFunc:_,nodeBorderRadius:a,nodeStrokeWidth:o,NodeComponent:l,onClick:c,shapeRendering:y},b))})}function j6({id:e,nodeColorFunc:t,nodeStrokeColorFunc:s,nodeClassNameFunc:a,nodeBorderRadius:o,nodeStrokeWidth:l,shapeRendering:c,NodeComponent:f,onClick:m}){const{node:p,x:_,y,width:b,height:v}=it(S=>{const w=S.nodeLookup.get(e);if(!w)return{node:void 0,x:0,y:0,width:0,height:0};const k=w.internals.userNode,{x:j,y:A}=w.internals.positionAbsolute,{width:N,height:T}=qs(k);return{node:k,x:j,y:A,width:N,height:T}},Lt);return!p||p.hidden||!k1(p)?null:d.jsx(f,{x:_,y,width:b,height:v,style:p.style,selected:!!p.selected,className:a(p),color:t(p),borderRadius:o,strokeColor:s(p),strokeWidth:l,shapeRendering:c,onClick:m,id:p.id})}const N6=C.memo(j6);var E6=C.memo(k6);const T6=200,C6=150,A6=e=>!e.hidden,O6=e=>{const t={x:-e.transform[0]/e.transform[2],y:-e.transform[1]/e.transform[2],width:e.width/e.transform[2],height:e.height/e.transform[2]};let s=!1;for(const a of e.nodeLookup.values())if(!a.hidden){s=!0;break}return{viewBB:t,boundingRect:s?x1(uc(e.nodeLookup,{filter:A6}),t):t,rfId:e.rfId,panZoom:e.panZoom,translateExtent:e.translateExtent,flowWidth:e.width,flowHeight:e.height,ariaLabelConfig:e.ariaLabelConfig}},Tx=(e,t)=>e.x===t.x&&e.y===t.y&&e.width===t.width&&e.height===t.height,M6=(e,t)=>Tx(e.viewBB,t.viewBB)&&Tx(e.boundingRect,t.boundingRect)&&e.rfId===t.rfId&&e.panZoom===t.panZoom&&e.translateExtent===t.translateExtent&&e.flowWidth===t.flowWidth&&e.flowHeight===t.flowHeight&&e.ariaLabelConfig===t.ariaLabelConfig,z6="react-flow__minimap-desc";function jk({style:e,className:t,nodeStrokeColor:s,nodeColor:a,nodeClassName:o="",nodeBorderRadius:l=5,nodeStrokeWidth:c,nodeComponent:f,bgColor:m,maskColor:p,maskStrokeColor:_,maskStrokeWidth:y,position:b="bottom-right",onClick:v,onNodeClick:S,pannable:w=!1,zoomable:k=!1,ariaLabel:j,inversePan:A,zoomStep:N=1,offsetScale:T=5}){const E=Et(),O=C.useRef(null),{boundingRect:R,panZoom:q,viewBB:Z,rfId:oe,translateExtent:Q,flowWidth:ee,flowHeight:X,ariaLabelConfig:L}=it(O6,M6),I=e?.width??T6,D=e?.height??C6,U=R.width/I,B=R.height/D,V=Math.max(U,B),te=V*I,z=V*D,Y=T*V,ce=R.x-(te-R.width)/2-Y,de=R.y-(z-R.height)/2-Y,_e=te+Y*2,F=z+Y*2,G=`${z6}-${oe}`,P=C.useRef(0),ae=C.useRef();P.current=V,C.useEffect(()=>{const xe=E.getState().panZoom;if(O.current&&xe)return ae.current=P3({domNode:O.current,panZoom:xe,getTransform:()=>E.getState().transform,getViewScale:()=>P.current}),()=>{ae.current?.destroy()}},[q]),C.useEffect(()=>{ae.current?.update({translateExtent:Q,width:ee,height:X,inversePan:A,pannable:w,zoomStep:N,zoomable:k})},[w,k,A,N,Q,ee,X]);const Se=v?xe=>{const[ke,me]=ae.current?.pointer(xe)||[0,0];v(xe,{x:ke,y:me})}:void 0,je=C.useCallback((xe,ke)=>{const me=E.getState().nodeLookup.get(ke).internals.userNode;S?.(xe,me)},[S]),we=S?je:void 0,ve=j??L["minimap.ariaLabel"];return d.jsx(Cr,{position:b,style:{...e,"--xy-minimap-background-color-props":typeof m=="string"?m:void 0,"--xy-minimap-mask-background-color-props":typeof p=="string"?p:void 0,"--xy-minimap-mask-stroke-color-props":typeof _=="string"?_:void 0,"--xy-minimap-mask-stroke-width-props":typeof y=="number"?y*V:void 0,"--xy-minimap-node-background-color-props":typeof a=="string"?a:void 0,"--xy-minimap-node-stroke-color-props":typeof s=="string"?s:void 0,"--xy-minimap-node-stroke-width-props":typeof c=="number"?c:void 0},className:an(["react-flow__minimap",t]),"data-testid":"rf__minimap",children:d.jsxs("svg",{width:I,height:D,viewBox:`${ce} ${de} ${_e} ${F}`,className:"react-flow__minimap-svg",role:"img","aria-labelledby":G,ref:O,onClick:Se,children:[ve&&d.jsx("title",{id:G,children:ve}),d.jsx(E6,{onClick:we,nodeColor:a,nodeStrokeColor:s,nodeBorderRadius:l,nodeClassName:o,nodeStrokeWidth:c,nodeComponent:f}),d.jsx("path",{className:"react-flow__minimap-mask",d:`M${ce-Y},${de-Y}h${_e+Y*2}v${F+Y*2}h${-_e-Y*2}z
        M${Z.x},${Z.y}h${Z.width}v${Z.height}h${-Z.width}z`,fillRule:"evenodd",pointerEvents:"none"})]})})}jk.displayName="MiniMap";const D6=C.memo(jk),$6=e=>t=>e?`${Math.max(1/t.transform[2],1)}`:void 0,R6={[Tr.Line]:"right",[Tr.Handle]:"bottom-right"};function L6({nodeId:e,position:t,variant:s=Tr.Handle,className:a,style:o=void 0,children:l,color:c,minWidth:f=10,minHeight:m=10,maxWidth:p=Number.MAX_VALUE,maxHeight:_=Number.MAX_VALUE,keepAspectRatio:y=!1,resizeDirection:b,autoScale:v=!0,shouldResize:S,onResizeStart:w,onResize:k,onResizeEnd:j}){const A=nk(),N=typeof e=="string"?e:A,T=Et(),E=C.useRef(null),O=s===Tr.Handle,R=it(C.useCallback($6(O&&v),[O,v]),Lt),q=C.useRef(null),Z=t??R6[s];C.useEffect(()=>{if(!(!E.current||!N))return q.current||(q.current=s4({domNode:E.current,nodeId:N,getStoreItems:()=>{const{nodeLookup:Q,transform:ee,snapGrid:X,snapToGrid:L,nodeOrigin:I,domNode:D}=T.getState();return{nodeLookup:Q,transform:ee,snapGrid:X,snapToGrid:L,nodeOrigin:I,paneDomNode:D}},onChange:(Q,ee)=>{const{triggerNodeChanges:X,nodeLookup:L,parentLookup:I,nodeOrigin:D}=T.getState(),U=[],B={x:Q.x,y:Q.y},V=L.get(N);if(V&&V.expandParent&&V.parentId){const te=V.origin??D,z=Q.width??V.measured.width??0,Y=Q.height??V.measured.height??0,ce={id:V.id,parentId:V.parentId,rect:{width:z,height:Y,...j1({x:Q.x??V.position.x,y:Q.y??V.position.y},{width:z,height:Y},V.parentId,L,te)}},de=wg([ce],L,I,D);U.push(...de),B.x=Q.x?Math.max(te[0]*z,Q.x):void 0,B.y=Q.y?Math.max(te[1]*Y,Q.y):void 0}if(B.x!==void 0&&B.y!==void 0){const te={id:N,type:"position",position:{...B}};U.push(te)}if(Q.width!==void 0&&Q.height!==void 0){const z={id:N,type:"dimensions",resizing:!0,setAttributes:b?b==="horizontal"?"width":"height":!0,dimensions:{width:Q.width,height:Q.height}};U.push(z)}for(const te of ee){const z={...te,type:"position"};U.push(z)}X(U)},onEnd:({width:Q,height:ee})=>{const X={id:N,type:"dimensions",resizing:!1,dimensions:{width:Q,height:ee}};T.getState().triggerNodeChanges([X])}})),q.current.update({controlPosition:Z,boundaries:{minWidth:f,minHeight:m,maxWidth:p,maxHeight:_},keepAspectRatio:y,resizeDirection:b,onResizeStart:w,onResize:k,onResizeEnd:j,shouldResize:S}),()=>{q.current?.destroy()}},[Z,f,m,p,_,y,w,k,j,S]);const oe=Z.split("-");return d.jsx("div",{className:an(["react-flow__resize-control","nodrag",...oe,s,a]),ref:E,style:{...o,scale:R,...c&&{[O?"backgroundColor":"borderColor"]:c}},children:l})}C.memo(L6);function q6(e){const t=e.toLowerCase();return t==="llm"||t.includes("agent")?{label:e,accent:"#6d4aff",tint:"#f3effe"}:t==="render_slides"||t==="render_study"?{label:e,accent:"#0b7a8f",tint:"#e8f7fa"}:t==="tts"||t==="render_audio"||t==="asr"?{label:e,accent:"#0d8a6a",tint:"#e7f8f2"}:t==="map"||t==="while"?{label:`${e} 并发子流`,accent:"#0f766e",tint:"#ecfdf5"}:t==="render_video"||t==="text_to_video"?{label:e,accent:"#c84b24",tint:"#fff1ec"}:t==="render_document"||t==="render_markdown"||t==="render_mindmap"||t==="render_table"||t==="figure"||t==="markmap"?{label:e,accent:"#3b5bdb",tint:"#edf2ff"}:t==="web_search"||t==="repo_snapshot"||t==="pdf_extract"||t==="video_ingest"||t==="arxiv"?{label:e,accent:"#b45309",tint:"#fef3c7"}:{label:e,accent:"#475569",tint:"#f1f5f9"}}const U6={idle:"待命",running:"运行中…",paused_before:"⏸ 执行前断点",paused_after:"⏸ 单步暂停 · 待核查",succeeded:"✓ 已完成",skipped:"↷ 条件跳过",failed:"✕ 失败"};function B6(e){return e<.001&&e>0?"<$0.001":`$${e.toFixed(e<.1?4:3)}`}function H6({data:e}){const{node:t,telemetry:s,hasBreakpoint:a,isSelected:o,editable:l,stepOrderIndex:c,onToggleBreakpoint:f,onSelectNode:m}=e,p=q6(t.type),_=t.sub_nodes?.length??0;return d.jsxs("div",{className:["studio-dag-node",`studio-dag-node--${s.status}`,o?"studio-dag-node--selected":"",l?"studio-dag-node--editable":""].filter(Boolean).join(" "),onClick:()=>m(t.id),role:"button",tabIndex:0,onKeyDown:y=>{(y.key==="Enter"||y.key===" ")&&(y.preventDefault(),m(t.id))},style:{"--node-accent":p.accent,"--node-tint":p.tint},children:[d.jsx(Ar,{type:"target",position:ze.Left,className:`studio-dag-handle ${l?"studio-dag-handle--interactive":""}`}),d.jsxs("div",{className:"studio-dag-node__top",children:[d.jsxs("span",{className:"studio-dag-node__step",children:["#",c+1]}),d.jsx("span",{className:"studio-dag-node__type",children:p.label}),d.jsx("button",{type:"button",className:`studio-bp-btn ${a?"studio-bp-btn--active":""}`,"aria-label":a?`取消节点 ${t.id} 断点`:`在节点 ${t.id} 设置执行前断点`,"aria-pressed":a,title:a?"已设执行前断点（点击移除）":"点击设置执行前断点",onClick:y=>{y.stopPropagation(),f(t.id)},children:d.jsx("span",{className:"studio-bp-dot"})})]}),d.jsx("div",{className:"studio-dag-node__title-row",children:d.jsx("strong",{className:"studio-dag-node__id",children:t.id})}),_>0&&d.jsxs("div",{className:"studio-dag-node__chip",children:["内嵌 ",_," 个子节点：",t.sub_nodes?.map(y=>`${y.id} (${y.type})`).join(", ")]}),t.run_if&&d.jsx("div",{className:"studio-dag-node__chip studio-dag-node__chip--cond",title:t.run_if,children:"run_if 条件守卫"}),d.jsxs("div",{className:"studio-dag-node__footer",children:[d.jsx("span",{className:`studio-node-status studio-node-status--${s.status}`,children:U6[s.status]}),d.jsxs("span",{className:"studio-dag-node__metrics",children:[typeof s.costUsd=="number"&&s.costUsd>0&&d.jsx("b",{className:"studio-cost-pill",title:`节点累计模型费用 $${s.costUsd.toFixed(6)}`,children:B6(s.costUsd)}),typeof s.durationMs=="number"&&d.jsxs("em",{children:[s.durationMs," ms"]}),(s.hasInput||s.hasOutput)&&d.jsx("b",{className:"studio-io-pill",title:"已捕获节点输入/输出快照",children:"I/O"})]})]}),d.jsx(Ar,{type:"source",position:ze.Right,className:`studio-dag-handle ${l?"studio-dag-handle--interactive":""}`})]})}const I6={studioNode:H6};function Z6({nodes:e,edges:t,levels:s,executionOrder:a,telemetryByNode:o,breakpoints:l,selectedNodeId:c,editable:f=!1,onSelectNode:m,onToggleBreakpoint:p,onAddNode:_,onConnectNodes:y,onDisconnectEdge:b,onDeleteNode:v}){const[S,w]=C.useState(!1),[k,j]=C.useState("LLM / Agent"),[A,N]=C.useState(!0),[T,E]=C.useState(null),O=C.useMemo(()=>new Set(l),[l]),R=C.useMemo(()=>{const X=new Map;return a.forEach((L,I)=>X.set(L,I)),X},[a]),q=C.useMemo(()=>{const I=Math.max(1,...s.map(U=>U.length)),D=new Map;return s.length>0&&s.forEach((U,B)=>{const V=(U.length-1)*166,z=((I-1)*166-V)/2+36;U.forEach((Y,ce)=>{D.set(Y,{x:B*304+36,y:z+ce*166})})}),e.map((U,B)=>{const V=D.get(U.id)??{x:B%4*304+36,y:Math.floor(B/4)*166+36},te=o[U.id]??{status:"idle"};return{id:U.id,type:"studioNode",position:V,selected:c===U.id,data:{node:U,telemetry:te,hasBreakpoint:O.has(U.id),isSelected:c===U.id,editable:f,stepOrderIndex:R.get(U.id)??B,onToggleBreakpoint:p,onSelectNode:z=>{E(null),m(z)}}}})},[e,s,o,O,c,f,R,p,m]),Z=C.useMemo(()=>t.map((X,L)=>{const I=o[X.from]?.status??"idle",D=o[X.to]?.status??"idle",U=X.kind==="condition",B=T?.from===X.from&&T?.to===X.to,V=D==="running"||D==="paused_before"||I==="paused_after",te=B?"#dc2626":U?"#d97706":I==="succeeded"?"#0d8a6a":V?"#2563eb":"#94a3b8";return{id:`e-${X.from}-${X.to}-${X.kind}-${L}`,source:X.from,target:X.to,type:"smoothstep",animated:V,selected:B,label:X.label??void 0,labelStyle:{fill:U?"#92400e":"#334155",fontSize:11,fontWeight:600},labelBgStyle:{fill:U?"#fef3c7":"#f8fafc",fillOpacity:.92},labelBgPadding:[5,3],labelBgBorderRadius:4,style:{stroke:te,strokeWidth:B?2.8:V||I==="succeeded"?2.2:1.7,strokeDasharray:U?"6 4":void 0},markerEnd:{type:Xl.ArrowClosed,color:te,width:16,height:16}}}),[t,o,T]),oe=C.useMemo(()=>Kd.filter(X=>X.category===k),[k]),Q=X=>{!f||!y||X.source&&X.target&&y(X.source,X.target)},ee=X=>{if(!_)return;_(X,A&&c?c:void 0),w(!1)};return e.length===0?d.jsxs("div",{className:"studio-canvas-empty",children:[d.jsx("p",{children:"当前 YAML 尚未解析出节点，请在左侧选择预设模板、修复语法错误，或直接添加首个节点。"}),f&&_&&d.jsxs("div",{className:"studio-empty-add-row",children:[d.jsx("button",{type:"button",className:"studio-btn-primary studio-btn-sm",onClick:()=>_("llm"),children:"+ 添加 LLM 节点起步"}),d.jsx("button",{type:"button",className:"studio-btn-secondary studio-btn-sm",onClick:()=>_("template"),children:"+ 添加 Template 节点起步"})]})]}):d.jsx("div",{className:"studio-canvas-wrap","data-testid":"workflow-graph-canvas",children:d.jsxs(l6,{nodes:q,edges:Z,nodeTypes:I6,fitView:!0,fitViewOptions:{padding:.18,maxZoom:1.15},minZoom:.25,maxZoom:1.8,nodesConnectable:f&&!!y,onConnect:Q,onEdgeClick:(X,L)=>{f&&E({from:L.source,to:L.target})},onPaneClick:()=>E(null),elementsSelectable:!0,proOptions:{hideAttribution:!0},children:[d.jsx(p6,{variant:Ci.Dots,gap:20,size:1.2,color:"#cbd5e1"}),d.jsx(v6,{showInteractive:!1,position:"bottom-left"}),d.jsx(D6,{pannable:!0,zoomable:!0,position:"bottom-right",nodeColor:X=>{const L=X.data?.telemetry?.status??"idle";return L==="paused_after"||L==="paused_before"?"#f59e0b":L==="running"?"#2563eb":L==="succeeded"?"#10b981":L==="failed"?"#ef4444":L==="skipped"?"#94a3b8":"#cbd5e1"}}),d.jsxs(Cr,{position:"top-left",className:"studio-canvas-legend",children:[d.jsxs("span",{children:[d.jsx("i",{className:"legend-line legend-line--data"})," 数据映射边"]}),d.jsxs("span",{children:[d.jsx("i",{className:"legend-line legend-line--cond"})," run_if 条件边"]}),d.jsxs("span",{children:[d.jsx("i",{className:"legend-bp"})," ",f?"拖拽节点右端口连线 · 点右上圆点设断点":"点击节点右上圆点设执行前断点"]})]}),f&&d.jsxs(Cr,{position:"top-right",className:"studio-canvas-builder-panel",children:[d.jsxs("div",{className:"studio-canvas-builder-bar",children:[T&&b&&d.jsxs("button",{type:"button","data-testid":"canvas-disconnect-edge-btn",className:"studio-btn-danger-ghost studio-btn-sm",onClick:()=>{b(T.from,T.to),E(null)},children:["✂ 断开 ",T.from," → ",T.to]}),c&&v&&e.length>1&&d.jsxs("button",{type:"button","data-testid":"canvas-delete-node-btn",className:"studio-btn-ghost studio-btn-sm",title:`删除当前选中节点 #${c} 并自动清理下游引用`,onClick:()=>v(c),children:["🗑 删除 #",c]}),_&&d.jsx("button",{type:"button","data-testid":"canvas-open-palette-btn",className:"studio-btn-primary studio-btn-sm","aria-expanded":S,onClick:()=>w(X=>!X),children:S?"✕ 收起组件库":"+ 添加节点"})]}),S&&_&&d.jsxs("div",{className:"studio-node-palette-drawer",role:"region","aria-label":"可视化添加工作流节点",children:[d.jsx("div",{className:"studio-node-palette__cats",children:rz.map(X=>d.jsx("button",{type:"button",className:`studio-cat-pill ${k===X?"active":""}`,onClick:()=>j(X),children:X},X))}),c&&d.jsxs("label",{className:"studio-palette-chain-toggle",children:[d.jsx("input",{type:"checkbox",checked:A,onChange:X=>N(X.target.checked)}),d.jsxs("span",{children:["自动连接在当前选中节点 #",c," 之后"]})]}),d.jsx("div",{className:"studio-node-palette__list",children:oe.map(X=>d.jsxs("button",{type:"button","data-testid":`palette-add-${X.type}`,className:"studio-palette-item",onClick:()=>ee(X.type),children:[d.jsx("strong",{children:X.label}),d.jsx("small",{children:X.description})]},X.type))})]})]})]})})}const P6=`# Y4.3 — \`audio_overview\` generator (docs/RFC_CONTENT_STUDIO.md §3, §4.3).
#
# Source (a URL, fetched with the http node's SSRF filter, or pasted
# text) → a two-voice script (llm, json_schema, one line per turn) →
# every line voiced in parallel (Map → tts, the speaker's voice) →
# joined and loudness-normalised (render_audio, ffmpeg) → one mp3 and a
# \`Deliverable\` carrying the script.
#
#   yanshi workflow run examples/generators/audio_overview.yml \\
#     --input url https://example.org/article \\
#     --input tts_model <tts model> --input host_voice <voice> \\
#     --input guest_voice <voice> --output-dir out
name: audio_overview
description: "A two-voice audio overview of a source, as one loudness-normalised mp3"

inputs:
  url:
    description: "Page to discuss (fetched with SSRF protection); leave empty to use \`text\`"
    default: ""
  text:
    description: "Source text, used when \`url\` is empty"
    default: ""
  focus:
    description: "What the conversation should cover"
    default: "the main ideas, in plain language"
  minutes:
    description: "Approximate length in minutes"
    default: 3
  language:
    description: "Language of the conversation"
    default: "English"
  tts_model:
    description: "Registry name of the text-to-speech model"
    default: "tts-1"
  host_voice:
    description: "Voice of the host"
    default: "alloy"
  guest_voice:
    description: "Voice of the guest"
    default: "nova"
  target_lufs:
    description: "Loudness target (podcast platforms: -16, broadcast: -23)"
    default: -16
  chunks:
    description: "Notebook passages to work from instead of url / text: [{chunk_id, source_id, content}]. Citations then point at these ids."
    default: []

nodes:
  - id: options
    type: template
    parameters:
      output_format: json
      template: '{"has_url": {{ url != "" }}}'

  # \`url\` (the workflow input) reaches the node directly.
  - id: fetch
    type: http
    dependencies: ["options"]
    run_if: "{{ nodes.options.outputs.has_url }}"

  - id: source
    type: template
    input_mapping:
      page: "{{ nodes.fetch.outputs.body }}"
    parameters:
      # With notebook \`chunks\`, every passage is labelled with its id so
      # the model can say where each quote comes from.
      template: "{% if chunks | length > 0 %}{% for c in chunks %}[{{ c.chunk_id }}] {{ c.content }}\\n\\n{% endfor %}{% elif page %}{{ page | striptags | truncate(length=24000) }}{% else %}{{ text | truncate(length=24000) }}{% endif %}"

  - id: script_prompt
    type: template
    dependencies: ["source"]
    input_mapping:
      source_text: "{{ nodes.source.outputs.output }}"
    parameters:
      template: |
        Write a {{ minutes }}-minute conversation in {{ language }} between a
        curious host and a knowledgeable guest about the source below,
        covering {{ focus }}. Natural spoken turns of one to three
        sentences, no stage directions, no sound effects. The host opens
        and closes. Every fact must come from the source; when its
        passages carry [ids], give each line that states a fact the
        chunk_id of its passage.

        SOURCE:
        {{ source_text }}

  - id: script
    type: llm
    dependencies: ["script_prompt"]
    input_mapping:
      prompt: "{{ nodes.script_prompt.outputs.output }}"
    parameters:
      system: "You write podcast scripts. Answer with JSON only."
      json_schema:
        type: object
        required: [title, lines]
        properties:
          title: { type: string }
          lines:
            type: array
            items:
              type: object
              required: [speaker, text]
              properties:
                speaker: { type: string, enum: [host, guest] }
                text: { type: string }
                chunk_id: { type: string }

  # One item per line, carrying what the tts node inside the Map needs
  # (a Map body sees only \`item\`, not the workflow inputs).
  - id: lines
    type: template
    dependencies: ["script"]
    input_mapping:
      plan: "{{ nodes.script.outputs.output }}"
    parameters:
      output_format: json
      template: |
        {"items": [{% for l in plan.lines %}{"text": {{ l.text | json_encode() }}, "voice": {% if l.speaker == "guest" %}{{ guest_voice | json_encode() }}{% else %}{{ host_voice | json_encode() }}{% endif %}, "model": {{ tts_model | json_encode() }}}{% if not loop.last %},{% endif %}{% endfor %}]}

  - id: voice_lines
    type: map
    dependencies: ["lines"]
    input_mapping:
      input_list: "{{ nodes.lines.outputs.items }}"
    parameters:
      parallel: true
      max_concurrent: 4
      template:
        - id: say
          type: tts
          input_mapping:
            input_template: "{{ item.text }}"
            voice: "{{ item.voice }}"
            model: "{{ item.model }}"

  # The clips, in script order.
  - id: clips
    type: template
    dependencies: ["voice_lines"]
    input_mapping:
      voiced: "{{ nodes.voice_lines.outputs.results }}"
    parameters:
      output_format: json
      template: |
        {"paths": [{% for r in voiced %}{{ r.say.Ok.say_output.path | json_encode() }}{% if not loop.last %},{% endif %}{% endfor %}]}

  - id: episode
    type: render_audio
    dependencies: ["clips"]
    input_mapping:
      clips: "{{ nodes.clips.outputs.paths }}"
    parameters:
      format: mp3

  - id: deliverable
    type: template
    dependencies: ["episode"]
    input_mapping:
      plan: "{{ nodes.script.outputs.output }}"
      mp3: "{{ nodes.episode.outputs.episode_output }}"
      lufs: "{{ nodes.episode.outputs.episode_lufs }}"
    parameters:
      output_format: json
      template: |
        {"deliverable": {
          "kind": "audio_overview",
          "title": {{ plan.title | json_encode() }},
          "content": {"script": {{ plan.lines | json_encode() }}, "loudness_lufs": {{ lufs | json_encode() }}},
          "citations": [{% set_global first = true %}{% for l in plan.lines %}{% if l.chunk_id %}{% set_global sid = "" %}{% set_global passage = "" %}{% for c in chunks %}{% if c.chunk_id == l.chunk_id %}{% set_global sid = c.source_id %}{% set_global passage = c.content %}{% endif %}{% endfor %}{% if passage %}{% if not first %},{% endif %}{% set_global first = false %}{"source_id": {{ sid | json_encode() }}, "chunk_id": {{ l.chunk_id | json_encode() }}, "quote": {{ passage | truncate(length=300) | json_encode() }}}{% endif %}{% endif %}{% endfor %}],
          "files": [
            {"node_id": "episode", "name": {{ mp3 | split(pat="/") | last | json_encode() }}, "mime_type": "audio/mpeg", "role": "primary"}
          ]
        }}
`,Y6=`# Daily Workflow 3 — \`backend_project_planner\`: Multi-Turn HITL Chat Clarification -> Backend Spec -> System Arch + Module Dep + UML Class + UML Sequence + Flowchart + DB ER + PostgreSQL DDL + Mindmap + PDF Spec.
#
#   yanshi workflow run examples/generators/backend_project_planner.yml \\
#     --input idea "设计一个支持高并发秒杀与多租户库存隔离的订单履约后端系统" --output-dir out
name: backend_project_planner
description: "通过多轮交互问答深入澄清用户想法（着重后端架构），自动生成系统架构图、模块依赖图、UML 类图与时序图套图、业务流程图、数据库 ER 图、PostgreSQL DDL、思维导图与详细设计说明书 PDF"

inputs:
  idea:
    description: "初始项目想法或业务需求简述"
    default: "设计一个支持高并发秒杀、多租户库存隔离与异步履约对账的电商订单后端系统"
  tech_preference:
    description: "后端技术栈偏好（如 Rust Axum + PostgreSQL + Redis + Kafka，留空则由架构师结合沟通推荐）"
    default: "Rust (Axum + Tokio + SQLx) + PostgreSQL + Redis + 消息队列"
  interactive:
    description: "是否开启多轮交互式澄清聊天（true 时在工作台弹出问答卡片向用户追问细节；false 时按行业最佳实践自动决策）"
    default: true
  max_questions:
    description: "多轮交互最多追问轮数（1-5）"
    default: 2
  export_pdf:
    description: "同时导出完整《后端架构与详细设计说明书》PDF"
    default: true

nodes:
  - id: ref_query
    type: template
    parameters:
      template: "{{ idea }} 后端系统架构 领域模型 高并发一致性 数据库设计 最佳实践"

  - id: industry_ref
    type: web_search
    dependencies: ["ref_query"]
    input_mapping:
      query: "{{ nodes.ref_query.outputs.output }}"
    parameters:
      max_results: 5

  - id: agent_brief
    type: template
    dependencies: ["industry_ref"]
    input_mapping:
      refs: "{{ nodes.industry_ref.outputs.markdown }}"
    parameters:
      template: |
        用户的初始项目构想如下：
        「{{ idea }}」
        技术栈倾向：{{ tech_preference }}

        你的任务是以首席后端系统架构师的身份，围绕后端核心边界（并发规模与读写比、核心业务闭环与一致性保障、多租户/鉴权隔离、关键外部集成或数据存储诉求）与用户充分对齐需求：
        - 如开启了交互模式且初始信息存在关键架构分叉点，请调用 \`ask_user\` 工具向用户提出聚焦、具体、易回答的澄清问题（每次合并提出最关键的 2-3 个子问题并提供推荐默认选项，最多不超过 {{ max_questions }} 轮，切勿反复追问琐碎细节）；
        - 在获得用户答复（或无需继续追问）后，综合行业参考实践，输出一份详尽、落地的《后端需求澄清与总体技术规格纪要》，涵盖：业务目标与非功能指标（QPS/延迟/可用性）、领域边界与核心聚合、服务/模块划分、核心 API 与异步事件流、状态机与异常补偿、数据库核心表规划。

        行业架构参考资料：
        {{ refs | truncate(length=10000) }}

  - id: clarify_requirements
    type: agent
    dependencies: ["agent_brief"]
    input_mapping:
      prompt: "{{ nodes.agent_brief.outputs.output }}"
    parameters:
      system: "你是资深后端系统架构师，擅长通过精炼的多轮沟通把模糊想法转化为严密的后端工程规格（高内聚低耦合、数据一致性、可观测性与演进路径）。"
      tools: ["web_search"]
      max_steps: 8

  - id: spec_prompt
    type: template
    dependencies: ["clarify_requirements"]
    input_mapping:
      clarified: "{{ nodes.clarify_requirements.outputs.output }}"
      qa_log: "{{ nodes.clarify_requirements.outputs.qa_history }}"
    parameters:
      template: |
        请基于以下与用户充分沟通确认后的《后端需求澄清与总体技术规格纪要》，输出结构化的后端工程设计蓝图 JSON，以便下游并行编译全套工程图谱与数据库脚本。

        要求（着重后端深度）：
        1. \`project_name\`、\`tagline\`、\`architecture_overview\`（详述网关接入、业务服务层、缓存/消息中间件、存储与可观测性设计）；
        2. \`non_functional_targets\`：并发吞吐、P99 延迟、一致性级别、高可用与容灾指标；
        3. \`backend_modules\`：4-8 个核心后端模块清单（\`name\`、\`layer\`、\`responsibility\`、\`dependencies\`、\`key_apis\`）；
        4. \`postgresql_ddl\`：核心 4-6 张业务表的精炼可执行生产级 PostgreSQL 建表 SQL（含主键、外键引用、唯一约束、关键复合索引与状态枚举检查，省略冗长注释）；
        5. \`mindmap_tree\`：项目全景思维导图树（根节点 \`label\` 为项目名，3 层以内分支涵盖：业务域划分、后端技术架构、核心接口与流程、数据库与缓存、稳定性与运维）；
        6. 为 6 张 D2 工程图分别提供精准、紧凑的结构描述（每项控制在 250 字以内，聚焦节点 ID、核心字段/方法与连线方向，不写冗余长篇解说）：
           - \`arch_d2_brief\`：系统总体架构图（网关、微服务/核心进程、缓存、MQ、DB、可观测体系分层容器与链路）；
           - \`module_d2_brief\`：后端模块依赖图（各业务模块/Crate/Package 之间的单向依赖拓扑，严禁循环依赖）；
           - \`uml_class_d2_brief\`：核心领域模型与服务接口 UML 类图（明确 4-6 个核心聚合根/实体/DomainService 的字段类型、方法签名与关联关系，使用 D2 \`shape: class\`）；
           - \`uml_seq_d2_brief\`：核心写路径业务链路 UML 时序图（如客户端 -> API 网关 -> 鉴权 -> 应用服务 -> Redis/DB/MQ 异步投递与回调，使用 D2 \`shape: sequence_diagram\`）；
           - \`flowchart_d2_brief\`：核心业务状态流转与异常幂等补偿流程图（含判定分支、成功闭环与失败回滚/死信重试分支）；
           - \`er_d2_brief\`：数据库物理表 ER 关系图（4-7 张核心业务表的主键 \`primary_key\`、外键 \`foreign_key\`、唯一约束 \`unique\` 及表间 \`table_a.col -> table_b.id\` 关系，与 \`postgresql_ddl\` 完全一致，使用 D2 \`shape: sql_table\`）。

        需求沟通问答记录：
        {{ qa_log | json_encode() }}

        确认后的架构纪要：
        {{ clarified }}

  - id: structure_spec
    type: llm
    dependencies: ["spec_prompt"]
    input_mapping:
      prompt: "{{ nodes.spec_prompt.outputs.output }}"
    parameters:
      max_tokens: 8192
      system: "你是严谨的首席后端架构师，仅输出符合 JSON Schema 的完整工程设计 JSON。"
      json_schema:
        type: object
        required:
          - project_name
          - tagline
          - architecture_overview
          - non_functional_targets
          - backend_modules
          - api_endpoints
          - postgresql_ddl
          - mindmap_tree
          - arch_d2_brief
          - module_d2_brief
          - uml_class_d2_brief
          - uml_seq_d2_brief
          - flowchart_d2_brief
          - er_d2_brief
        properties:
          project_name: { type: string }
          tagline: { type: string }
          architecture_overview: { type: string }
          non_functional_targets:
            type: array
            maxItems: 10
            items: { type: string }
          backend_modules:
            type: array
            minItems: 2
            maxItems: 10
            items:
              type: object
              required: [name, layer, responsibility, dependencies, key_apis]
              properties:
                name: { type: string }
                layer: { type: string }
                responsibility: { type: string }
                dependencies:
                  type: array
                  items: { type: string }
                key_apis:
                  type: array
                  items: { type: string }
          api_endpoints:
            type: array
            maxItems: 12
            items:
              type: object
              required: [method, path, summary, idempotency_and_auth]
              properties:
                method: { type: string }
                path: { type: string }
                summary: { type: string }
                idempotency_and_auth: { type: string }
          postgresql_ddl: { type: string }
          mindmap_tree:
            type: object
            required: [label, children]
            properties:
              label: { type: string }
              children:
                type: array
                items:
                  type: object
                  required: [label]
                  properties:
                    label: { type: string }
                    children:
                      type: array
                      items:
                        type: object
                        required: [label]
                        properties:
                          label: { type: string }
                          children:
                            type: array
                            items:
                              type: object
                              required: [label]
                              properties:
                                label: { type: string }
          arch_d2_brief: { type: string }
          module_d2_brief: { type: string }
          uml_class_d2_brief: { type: string }
          uml_seq_d2_brief: { type: string }
          flowchart_d2_brief: { type: string }
          er_d2_brief: { type: string }

  - id: arch_desc
    type: template
    dependencies: ["structure_spec"]
    input_mapping:
      spec: "{{ nodes.structure_spec.outputs.output }}"
    parameters:
      template: |
        绘制项目「{{ spec.project_name }}」的后端系统总体架构图（D2 嵌套容器分层，direction: down，清晰区分接入网关层、核心领域服务层、异步消息与缓存层、持久化存储与可观测层）：
        {{ spec.arch_d2_brief }}

  - id: arch_diagram
    type: figure
    dependencies: ["arch_desc"]
    input_mapping:
      description: "{{ nodes.arch_desc.outputs.output }}"
    parameters:
      language: d2
      png: true
      png_width: 1800
      max_attempts: 4

  - id: module_desc
    type: template
    dependencies: ["structure_spec"]
    input_mapping:
      spec: "{{ nodes.structure_spec.outputs.output }}"
    parameters:
      template: |
        绘制项目「{{ spec.project_name }}」的后端模块依赖拓扑图（D2 有向无环图，direction: right，展示各层模块及单向依赖边）：
        {{ spec.module_d2_brief }}

  - id: module_diagram
    type: figure
    dependencies: ["module_desc"]
    input_mapping:
      description: "{{ nodes.module_desc.outputs.output }}"
    parameters:
      language: d2
      png: true
      png_width: 1800
      max_attempts: 4

  - id: uml_class_desc
    type: template
    dependencies: ["structure_spec"]
    input_mapping:
      spec: "{{ nodes.structure_spec.outputs.output }}"
    parameters:
      template: |
        使用 D2 的 \`shape: class\` 语法绘制项目「{{ spec.project_name }}」的核心领域聚合与后端服务 UML 类图（字段类型与方法签名必须用双引号包裹键值，并用有向边标注聚合/组合/调用关系）：
        {{ spec.uml_class_d2_brief }}

  - id: uml_class_diagram
    type: figure
    dependencies: ["uml_class_desc"]
    input_mapping:
      description: "{{ nodes.uml_class_desc.outputs.output }}"
    parameters:
      language: d2
      png: true
      png_width: 1800
      max_attempts: 4

  - id: uml_seq_desc
    type: template
    dependencies: ["structure_spec"]
    input_mapping:
      spec: "{{ nodes.structure_spec.outputs.output }}"
    parameters:
      template: |
        使用 D2 的 \`shape: sequence_diagram\` 语法绘制项目「{{ spec.project_name }}」的核心后端接口与异步事务 UML 时序图（先声明各参与者角色，再按时间顺序编写请求、校验、落库、缓存/消息投递与响应消息）：
        {{ spec.uml_seq_d2_brief }}

  - id: uml_sequence_diagram
    type: figure
    dependencies: ["uml_seq_desc"]
    input_mapping:
      description: "{{ nodes.uml_seq_desc.outputs.output }}"
    parameters:
      language: d2
      png: true
      png_width: 1800
      max_attempts: 4

  - id: flowchart_desc
    type: template
    dependencies: ["structure_spec"]
    input_mapping:
      spec: "{{ nodes.structure_spec.outputs.output }}"
    parameters:
      template: |
        使用 D2 绘制项目「{{ spec.project_name }}」的核心业务状态流转与异常幂等补偿流程图（direction: down，判定节点使用 \`shape: diamond\`，起止节点使用 \`shape: oval\`，清晰标出成功主链路与超时/库存不足/对账补偿分支）：
        {{ spec.flowchart_d2_brief }}

  - id: business_flowchart
    type: figure
    dependencies: ["flowchart_desc"]
    input_mapping:
      description: "{{ nodes.flowchart_desc.outputs.output }}"
    parameters:
      language: d2
      png: true
      png_width: 1800
      max_attempts: 4

  - id: er_desc
    type: template
    dependencies: ["structure_spec"]
    input_mapping:
      spec: "{{ nodes.structure_spec.outputs.output }}"
    parameters:
      template: |
        使用 D2 的 \`shape: sql_table\` 语法绘制项目「{{ spec.project_name }}」的数据库物理表 ER 关系图（每张表列出真实字段与类型，标注 \`{constraint: primary_key}\`、\`{constraint: foreign_key}\`、\`{constraint: unique}\`，并用 \`child_table.fk_col -> parent_table.id\` 绘制外键关联连线）：
        {{ spec.er_d2_brief }}

  - id: db_er_diagram
    type: figure
    dependencies: ["er_desc"]
    input_mapping:
      description: "{{ nodes.er_desc.outputs.output }}"
    parameters:
      language: d2
      png: true
      png_width: 1800
      max_attempts: 4

  - id: mindmap_extract
    type: template
    dependencies: ["structure_spec"]
    input_mapping:
      spec: "{{ nodes.structure_spec.outputs.output }}"
    parameters:
      output_format: json
      template: '{"tree": {{ spec.mindmap_tree | json_encode() }}}'

  - id: mindmap_svg
    type: render_mindmap
    dependencies: ["mindmap_extract"]
    input_mapping:
      tree: "{{ nodes.mindmap_extract.outputs.tree }}"

  - id: mindmap_png
    type: render_svg
    dependencies: ["mindmap_svg"]
    input_mapping:
      svg: "{{ nodes.mindmap_svg.outputs.mindmap_svg_output }}"
    parameters:
      width: 1600

  - id: mindmap_html
    type: render_mindmap
    dependencies: ["mindmap_extract"]
    input_mapping:
      tree: "{{ nodes.mindmap_extract.outputs.tree }}"
    parameters:
      format: html

  - id: assemble_spec_doc
    type: template
    dependencies:
      - clarify_requirements
      - structure_spec
      - arch_diagram
      - module_diagram
      - uml_class_diagram
      - uml_sequence_diagram
      - business_flowchart
      - db_er_diagram
      - mindmap_png
    input_mapping:
      qa_log: "{{ nodes.clarify_requirements.outputs.qa_history }}"
      spec: "{{ nodes.structure_spec.outputs.output }}"
      arch_png: "{{ nodes.arch_diagram.outputs.arch_diagram_png }}"
      mod_png: "{{ nodes.module_diagram.outputs.module_diagram_png }}"
      uml_cls_png: "{{ nodes.uml_class_diagram.outputs.uml_class_diagram_png }}"
      uml_seq_png: "{{ nodes.uml_sequence_diagram.outputs.uml_sequence_diagram_png }}"
      flow_png: "{{ nodes.business_flowchart.outputs.business_flowchart_png }}"
      er_png: "{{ nodes.db_er_diagram.outputs.db_er_diagram_png }}"
      mm_png: "{{ nodes.mindmap_png.outputs.mindmap_png_output }}"
    parameters:
      template: |
        # {{ spec.project_name }} — 后端架构与全套工程图设计说明书

        > **项目定位**：{{ spec.tagline }}
        > **推荐技术栈**：{{ tech_preference }}

        {% if qa_log and qa_log | length > 0 %}
        ## 1. 需求澄清与架构决策记录

        {% for item in qa_log %}
        - **架构师追问 {{ item.round }}**：{{ item.question }}
          - **确认决策**：{{ item.answer }}
        {% endfor %}
        {% endif %}

        ## 2. 项目全景思维导图

        ![项目全景思维导图]({{ mm_png }})

        ## 3. 总体后端系统架构设计

        {{ spec.architecture_overview }}

        ### 非功能与稳定性指标
        {% for t in spec.non_functional_targets %}- {{ t }}
        {% endfor %}

        ![后端系统总体架构图]({{ arch_png }})

        ## 4. 后端模块划分与依赖拓扑

        ![后端模块依赖图]({{ mod_png }})

        | 模块名称 | 所属分层 | 核心职责 | 依赖模块 | 代表接口/组件 |
        | --- | --- | --- | --- | --- |
        {% for m in spec.backend_modules %}| \`{{ m.name }}\` | {{ m.layer }} | {{ m.responsibility }} | {% if m.dependencies | length > 0 %}{{ m.dependencies | join(sep=", ") }}{% else %}—{% endif %} | {{ m.key_apis | join(sep="; ") }} |
        {% endfor %}

        ## 5. 领域模型与后端接口设计（UML 套图）

        ### 5.1 核心领域聚合与服务 UML 类图

        ![UML 类图]({{ uml_cls_png }})

        ### 5.2 核心业务链路 UML 时序图

        ![UML 时序图]({{ uml_seq_png }})

        ### 5.3 核心后端 REST / RPC 接口契约

        | 方法 | 路径 | 功能说明 | 鉴权与幂等保障 |
        | --- | --- | --- | --- |
        {% for api in spec.api_endpoints %}| \`{{ api.method }}\` | \`{{ api.path }}\` | {{ api.summary }} | {{ api.idempotency_and_auth }} |
        {% endfor %}

        ## 6. 核心业务状态流转与异常补偿流程图

        ![业务与状态补偿流程图]({{ flow_png }})

        ## 7. 数据库 ER 图与生产级 PostgreSQL DDL

        ### 7.1 数据库物理表 ER 关系图

        ![数据库 ER 关系图]({{ er_png }})

        ### 7.2 PostgreSQL 建表与索引脚本

        \`\`\`sql
        {{ spec.postgresql_ddl }}
        \`\`\`

  - id: spec_md
    type: render_markdown
    dependencies: ["assemble_spec_doc"]
    input_mapping:
      markdown: "{{ nodes.assemble_spec_doc.outputs.output }}"
    parameters:
      format: md

  - id: spec_html
    type: render_markdown
    dependencies: ["assemble_spec_doc"]
    input_mapping:
      markdown: "{{ nodes.assemble_spec_doc.outputs.output }}"

  - id: pdf_flag
    type: template
    parameters:
      output_format: json
      template: '{"want_pdf": {{ export_pdf }}}'

  - id: spec_pdf
    type: render_document
    dependencies: ["assemble_spec_doc", "pdf_flag"]
    run_if: "{{ nodes.pdf_flag.outputs.want_pdf }}"
    input_mapping:
      markdown: "{{ nodes.assemble_spec_doc.outputs.output }}"
    parameters:
      format: pdf
      toc: true

  - id: deliverable
    type: template
    dependencies:
      - structure_spec
      - arch_diagram
      - module_diagram
      - uml_class_diagram
      - uml_sequence_diagram
      - business_flowchart
      - db_er_diagram
      - mindmap_svg
      - mindmap_html
      - spec_md
      - spec_html
      - spec_pdf
    input_mapping:
      spec: "{{ nodes.structure_spec.outputs.output }}"
      arch_svg: "{{ nodes.arch_diagram.outputs.arch_diagram_svg }}"
      mod_svg: "{{ nodes.module_diagram.outputs.module_diagram_svg }}"
      cls_svg: "{{ nodes.uml_class_diagram.outputs.uml_class_diagram_svg }}"
      seq_svg: "{{ nodes.uml_sequence_diagram.outputs.uml_sequence_diagram_svg }}"
      flow_svg: "{{ nodes.business_flowchart.outputs.business_flowchart_svg }}"
      er_svg: "{{ nodes.db_er_diagram.outputs.db_er_diagram_svg }}"
      mm_svg: "{{ nodes.mindmap_svg.outputs.mindmap_svg_output }}"
      mm_html: "{{ nodes.mindmap_html.outputs.mindmap_html_output }}"
      md: "{{ nodes.spec_md.outputs.spec_md_output }}"
      html: "{{ nodes.spec_html.outputs.spec_html_output }}"
      pdf: "{{ nodes.spec_pdf.outputs.spec_pdf_output }}"
    parameters:
      output_format: json
      template: |
        {"deliverable": {
          "kind": "backend_project_plan",
          "title": {{ spec.project_name | json_encode() }},
          "content": {{ spec | json_encode() }},
          "citations": [],
          "files": [
            {"node_id": "arch_diagram", "name": {{ arch_svg | split(pat="/") | last | json_encode() }}, "mime_type": "image/svg+xml", "role": "primary"},
            {"node_id": "module_diagram", "name": {{ mod_svg | split(pat="/") | last | json_encode() }}, "mime_type": "image/svg+xml", "role": "primary"},
            {"node_id": "uml_class_diagram", "name": {{ cls_svg | split(pat="/") | last | json_encode() }}, "mime_type": "image/svg+xml", "role": "primary"},
            {"node_id": "uml_sequence_diagram", "name": {{ seq_svg | split(pat="/") | last | json_encode() }}, "mime_type": "image/svg+xml", "role": "primary"},
            {"node_id": "business_flowchart", "name": {{ flow_svg | split(pat="/") | last | json_encode() }}, "mime_type": "image/svg+xml", "role": "primary"},
            {"node_id": "db_er_diagram", "name": {{ er_svg | split(pat="/") | last | json_encode() }}, "mime_type": "image/svg+xml", "role": "primary"},
            {"node_id": "mindmap_svg", "name": {{ mm_svg | split(pat="/") | last | json_encode() }}, "mime_type": "image/svg+xml", "role": "asset"},
            {"node_id": "mindmap_html", "name": {{ mm_html | split(pat="/") | last | json_encode() }}, "mime_type": "text/html", "role": "preview"},
            {"node_id": "spec_md", "name": {{ md | split(pat="/") | last | json_encode() }}, "mime_type": "text/markdown", "role": "asset"},
            {"node_id": "spec_html", "name": {{ html | split(pat="/") | last | json_encode() }}, "mime_type": "text/html", "role": "preview"}{% if pdf %},
            {"node_id": "spec_pdf", "name": {{ pdf | split(pat="/") | last | json_encode() }}, "mime_type": "application/pdf", "role": "primary"}{% endif %}
          ]
        }}
`,V6=`# Y6.5 — domain engineering diagram: an electrical schematic
# (docs/RFC_CONTENT_STUDIO.md §5.3).
#
# A circuit described in words → a parts list (llm, json_schema: id,
# kind, value per component) → \`figure\` with \`domain: circuit\`: the
# model writes circuitikz, the source is checked against the parts list
# (every component drawn as its kind and labelled with its id) and then
# compiled (lualatex → pdftocairo); a missing part or a compile error is
# fed back for another attempt → SVG + PNG → a \`Deliverable\`.
#
#   yanshi workflow run examples/generators/circuit_diagram.yml \\
#     --input description "an RC low-pass filter driven by a 5 V source" \\
#     --output-dir out
#
# Needs TeX Live (with circuitikz) and poppler (\`yanshi doctor\`).
name: circuit_diagram
description: "An electrical schematic from a description, checked against its parts list, as SVG and PNG"

inputs:
  description:
    description: "The circuit, in words: what it does and which parts it has"
  text:
    description: "Optional source text the circuit is described in"
    default: ""

nodes:
  - id: parts_prompt
    type: template
    parameters:
      template: |
        List the components of this circuit.
        {{ description }}
        {% if text %}
        Source:
        {{ text | truncate(length=12000) }}
        {% endif %}
        Give every component an id of a letter and a number (R1, C1, V1,
        D1, ...), its kind, and its value when one is known. Add one
        \`ground\` entry when the circuit is grounded. \`brief\` describes the
        topology: what connects to what.

  - id: parts
    type: llm
    dependencies: ["parts_prompt"]
    input_mapping:
      prompt: "{{ nodes.parts_prompt.outputs.output }}"
    parameters:
      system: "You are an electrical engineer. Answer with JSON only."
      json_schema:
        type: object
        required: [title, brief, components]
        properties:
          title: { type: string }
          brief: { type: string }
          components:
            type: array
            minItems: 1
            items:
              type: object
              required: [kind]
              properties:
                id: { type: string }
                kind:
                  type: string
                  enum: [resistor, capacitor, inductor, voltage_source, current_source, diode, led, switch, lamp, ammeter, voltmeter, ground]
                value: { type: string }

  - id: plan
    type: template
    dependencies: ["parts"]
    input_mapping:
      p: "{{ nodes.parts.outputs.output }}"
    parameters:
      output_format: json
      template: '{"spec": {{ p.components | json_encode() }}, "brief": {{ p.brief | json_encode() }}}'

  - id: schematic
    type: figure
    dependencies: ["plan"]
    input_mapping:
      spec: "{{ nodes.plan.outputs.spec }}"
      description: "{{ nodes.plan.outputs.brief }}"
    parameters:
      language: tikz
      domain: circuit
      png: true
      png_width: 1600

  - id: deliverable
    type: template
    dependencies: ["schematic"]
    input_mapping:
      p: "{{ nodes.parts.outputs.output }}"
      svg: "{{ nodes.schematic.outputs.schematic_svg }}"
      png: "{{ nodes.schematic.outputs.schematic_png }}"
      source: "{{ nodes.schematic.outputs.schematic_source }}"
      attempts: "{{ nodes.schematic.outputs.schematic_attempts }}"
    parameters:
      output_format: json
      template: |
        {"deliverable": {
          "kind": "figure",
          "title": {{ p.title | json_encode() }},
          "content": {"domain": "circuit", "components": {{ p.components | json_encode() }}, "source": {{ source | json_encode() }}, "attempts": {{ attempts }}},
          "citations": [],
          "files": [
            {"node_id": "schematic", "name": {{ svg | split(pat="/") | last | json_encode() }}, "mime_type": "image/svg+xml", "role": "primary"},
            {"node_id": "schematic", "name": {{ png | split(pat="/") | last | json_encode() }}, "mime_type": "image/png", "role": "preview"}
          ]
        }}
`,K6=`# Y6.2 — \`flashcards\` generator (docs/RFC_CONTENT_STUDIO.md §3).
#
# Source (a URL, fetched with the http node's SSRF filter, pasted text,
# or notebook chunks) → cards (llm, json_schema) → one self-contained
# HTML page you click through (render_study) → one \`Deliverable\` whose
# content carries the cards as JSON.
#
#   yanshi workflow run examples/generators/flashcards.yml \\
#     --input url https://example.org/article --output-dir out
name: flashcards
description: "Flashcards from a source: JSON cards and a click-to-flip HTML page"

inputs:
  url:
    description: "Page to study (fetched with SSRF protection); leave empty to use \`text\`"
    default: ""
  text:
    description: "Source text, used when \`url\` is empty"
    default: ""
  focus:
    description: "What the cards should test"
    default: "the key facts and ideas"
  count:
    description: "How many cards (5-30)"
    default: 12
  chunks:
    description: "Notebook passages to work from instead of url / text: [{chunk_id, source_id, content}]. Citations then point at these ids."
    default: []

nodes:
  - id: options
    type: template
    parameters:
      output_format: json
      template: '{"has_url": {{ url != "" }}}'

  # \`url\` (the workflow input) reaches the node directly.
  - id: fetch
    type: http
    dependencies: ["options"]
    run_if: "{{ nodes.options.outputs.has_url }}"

  - id: source
    type: template
    input_mapping:
      page: "{{ nodes.fetch.outputs.body }}"
    parameters:
      template: "{% if chunks | length > 0 %}{% for c in chunks %}[{{ c.chunk_id }}] {{ c.content }}\\n\\n{% endfor %}{% elif page %}{{ page | striptags | truncate(length=24000) }}{% else %}{{ text | truncate(length=24000) }}{% endif %}"

  - id: prompt
    type: template
    dependencies: ["source"]
    input_mapping:
      source_text: "{{ nodes.source.outputs.output }}"
    parameters:
      template: |
        Write {{ count }} flashcards on the source below, covering {{ focus }}.
        \`front\` is a short question or term; \`back\` answers it in at most
        two sentences. Use only what the source supports, one fact per
        card, no duplicates. When the source's passages carry [ids], give
        every card the chunk_id of the passage it comes from.

        SOURCE:
        {{ source_text }}

  - id: draft
    type: llm
    dependencies: ["prompt"]
    input_mapping:
      prompt: "{{ nodes.prompt.outputs.output }}"
    parameters:
      system: "You write precise study flashcards. Answer with JSON only."
      json_schema:
        type: object
        required: [title, items]
        properties:
          title: { type: string }
          items:
            type: array
            minItems: 1
            items:
              type: object
              required: [front, back]
              properties:
                front: { type: string }
                back: { type: string }
                chunk_id: { type: string }

  # Lift the items out of the model's answer for the renderer.
  - id: lifted
    type: template
    dependencies: ["draft"]
    input_mapping:
      draft: "{{ nodes.draft.outputs.output }}"
    parameters:
      output_format: json
      template: '{"title": {{ draft.title | json_encode() }}, "items": {{ draft["items"] | json_encode() }}}'

  - id: page
    type: render_study
    dependencies: ["lifted"]
    input_mapping:
      title: "{{ nodes.lifted.outputs.title }}"
      items: "{{ nodes.lifted.outputs.items }}"
    parameters:
      kind: flashcards

  - id: deliverable
    type: template
    dependencies: ["page"]
    input_mapping:
      drafted: "{{ nodes.draft.outputs.output }}"
      html: "{{ nodes.page.outputs.page_output }}"
    parameters:
      output_format: json
      template: |
        {"deliverable": {
          "kind": "flashcards",
          "title": {{ drafted.title | json_encode() }},
          "content": {"items": {{ drafted["items"] | json_encode() }}},
          "citations": [{% set_global first = true %}{% for n in drafted["items"] %}{% if n.chunk_id %}{% set_global sid = "" %}{% set_global passage = "" %}{% for c in chunks %}{% if c.chunk_id == n.chunk_id %}{% set_global sid = c.source_id %}{% set_global passage = c.content %}{% endif %}{% endfor %}{% if passage %}{% if not first %},{% endif %}{% set_global first = false %}{"source_id": {{ sid | json_encode() }}, "chunk_id": {{ n.chunk_id | json_encode() }}, "quote": {{ passage | truncate(length=300) | json_encode() }}}{% endif %}{% endif %}{% endfor %}],
          "files": [
            {"node_id": "page", "name": {{ html | split(pat="/") | last | json_encode() }}, "mime_type": "text/html", "role": "primary"}
          ]
        }}
`,G6=`# Daily Workflow 1 — \`illustrated_slides\`: Topic Research -> Outline -> Expanded Content -> AI Illustrations & D2 Diagram -> Keynote HTML + PPTX + PDF.
#
#   yanshi workflow run examples/generators/illustrated_slides.yml \\
#     --input subject "Rust 异步运行时 Tokio 调度器与工作窃取原理" --output-dir out
name: illustrated_slides
description: "根据主题联网搜集资料、规划 PPT 大纲、按讲解风格扩写内容，自动生成精美插图与 D2 架构图，输出交互式 HTML 演示、可编辑 PPTX 与 PDF 讲义"

inputs:
  subject:
    description: "PPT 演讲主题"
    default: "Rust 异步运行时 Tokio 调度器与工作窃取原理"
  audience:
    description: "目标听众群体"
    default: "后端研发工程师与系统架构师"
  narration_style:
    description: "讲解风格与表达基调（如深入浅出、工程实战、学术严谨、产品路演）"
    default: "深入浅出、类比直观、结合真实高并发工程案例"
  slide_count:
    description: "幻灯片章节页数（4-10）"
    default: 6
  enable_web_search:
    description: "是否联网检索最新权威技术资料支撑论点"
    default: true
  slide_theme:
    description: "幻灯片视觉配色主题：'dark' 或 'light'"
    default: "dark"
  style_preset:
    description: "插图艺术指导预设：'editorial_tech' | 'isometric_3d' | 'flat_vector' | 'blueprint_schematic' | 'scientific_diagram' | 'cinematic_keynote'"
    default: "editorial_tech"
  visual_style:
    description: "补充视觉风格细节（与 style_preset 叠加生效）"
    default: "subtle isometric depth, harmonious sapphire and warm amber accents, crisp studio lighting"
  refine_prompt:
    description: "是否先经艺术总监模型精炼视觉隐喻提示词"
    default: true
  enable_illustrations:
    description: "是否调用精美插图节点生成定制配图（设为 false 时仅使用 D2 矢量架构图与结构化面板）"
    default: true
  image_model:
    description: "文生图模型名称（需在 models.yml 配置对应厂商密钥）"
    default: "gemini-3.1-flash-image"
  image_size:
    description: "生成插图尺寸或画幅比例（自动跨厂商归一化）"
    default: "16:9"
  export_pptx:
    description: "导出原生 Office OpenXML 可编辑 PowerPoint (.pptx)"
    default: true
  export_pdf:
    description: "导出 16:9 宽屏高清排版讲义 (.pdf)"
    default: true

nodes:
  - id: options
    type: template
    parameters:
      output_format: json
      template: |
        {
          "do_search": {{ enable_web_search }},
          "slide_theme": {{ slide_theme | json_encode() }},
          "d2_theme": {% if slide_theme == "dark" %}"200"{% else %}"0"{% endif %},
          "visual_style": {{ visual_style | json_encode() }},
          "enable_illustrations": {{ enable_illustrations }},
          "image_model": {{ image_model | json_encode() }},
          "image_size": {{ image_size | json_encode() }}
        }

  - id: search_query
    type: template
    parameters:
      template: "{{ subject }} 核心原理 架构设计 关键指标 最佳实践"

  - id: research
    type: web_search
    dependencies: ["options", "search_query"]
    run_if: "{{ nodes.options.outputs.do_search }}"
    input_mapping:
      query: "{{ nodes.search_query.outputs.output }}"
    parameters:
      max_results: 6

  - id: outline_prompt
    type: template
    dependencies: ["options"]
    input_mapping:
      search_notes: "{{ nodes.research.outputs.markdown }}"
    parameters:
      template: |
        你是一位兼具系统深度与视觉审美力的首席演讲策划师。请围绕主题「{{ subject }}」，面向「{{ audience }}」，按照「{{ narration_style }}」的讲解方式，规划一份共 {{ slide_count }} 页的精品 PPT 大纲。

        大纲规划要求：
        1. 叙事弧线清晰：第 1 页破题立意与痛点引入，中间页层层递进拆解核心架构/机制/工程权衡，最后 1 页给出落地最佳实践总结。
        2. 为每一页明确规划右侧视觉呈现模式 \`visual_mode\`：
           - \`"illustration"\`：适合概念隐喻、开篇引入、场景故事或总结升华页（预计安排 2-3 页），需提供精确的英文纯视觉画面描述 \`illustration_prompt\`（严禁出现任何文字、字母、标语）；
           - \`"arch_diagram"\`：全套课件安排恰好 1 页最核心的系统架构或数据流机制页，由 D2 编译器绘制高精度工程矢量架构图；
           - \`"structured_panel"\`：其余硬核拆解页使用双栏工程可视化对照（代码批注、对比矩阵表格、内存布局或思维导图）。
        3. 充分利用以下联网检索到的背景资料与真实技术细节支撑论点：

        {{ search_notes | default(value="（已启用离线领域知识模式）") | truncate(length=14000) }}

  - id: plan_outline
    type: llm
    dependencies: ["outline_prompt"]
    input_mapping:
      prompt: "{{ nodes.outline_prompt.outputs.output }}"
    parameters:
      max_tokens: 8192
      system: "你是严谨的 PPT 大纲架构师，仅输出符合 JSON Schema 的有效 JSON。"
      json_schema:
        type: object
        required: [title, subtitle, arch_diagram_spec, chapters]
        properties:
          title: { type: string }
          subtitle: { type: string }
          arch_diagram_spec:
            type: string
            description: "用自然语言列出核心架构图的 2 个分层与 5-7 个核心组件及其调用流向（切勿在此处写 D2 代码语法）"
          chapters:
            type: array
            minItems: 3
            maxItems: 12
            items:
              type: object
              required: [section, title, key_points, narration_angle, visual_mode]
              properties:
                section: { type: string }
                title: { type: string }
                key_points:
                  type: array
                  items: { type: string }
                narration_angle: { type: string }
                visual_mode:
                  type: string
                  enum: [illustration, arch_diagram, structured_panel]
                illustration_prompt: { type: string }

  - id: arch_figure_desc
    type: template
    dependencies: ["plan_outline"]
    input_mapping:
      outline: "{{ nodes.plan_outline.outputs.output }}"
    parameters:
      template: |
        为主题「{{ outline.title }}」绘制一张紧凑清晰的 D2 系统架构图（\`direction: right\`，总计 5-8 个核心节点；若需分组容器请直接写 \`group_id: "分组标题" { node_a: "组件名" }\`，严禁使用不存在的 \`container\` 关键字；跨容器连线必须写在所有 \`{}\` 之外的顶层并带完整路径，如 \`user_space.app -> kernel_space.ring: "提交 SQE"\`）：
        {{ outline.arch_diagram_spec }}

  - id: arch_figure
    type: figure
    dependencies: ["arch_figure_desc", "options"]
    input_mapping:
      description: "{{ nodes.arch_figure_desc.outputs.output }}"
      d2_theme: "{{ nodes.options.outputs.d2_theme }}"
    parameters:
      language: d2
      png: true
      png_width: 1600
      max_attempts: 4

  - id: expand_prompt
    type: template
    dependencies: ["plan_outline"]
    input_mapping:
      outline: "{{ nodes.plan_outline.outputs.output }}"
      search_notes: "{{ nodes.research.outputs.markdown }}"
    parameters:
      template: |
        请基于以下已确认的 PPT 大纲与检索资料，将每一章扩写为完整、可直接渲染的结构化幻灯片内容（目标听众：{{ audience }}；讲解方式：{{ narration_style }}；配色主题：{{ slide_theme }}）。

        填充规范（极其重要）：
        1. 保持与大纲完全一致的页数与章节顺序，每页必须包含：
           - \`section\`（如 "01 痛点破局"）、\`title\`、\`subtitle\`
           - \`left\`：左侧主内容面板，必须符合 \`SlideDeck\` 面板结构之一：
             * \`{"kind": "bullets", "title": "...", "grid": false, "items": [{"badge": "核心", "heading": "...", "body": "...", "tone": "accent"}]}\`（每页 3-4 条干货要点，避免空话）
             * \`{"kind": "code", "title": "...", "language": "rust", "code": "...", "callouts": [{"line": 3, "text": "...", "tone": "cyan"}]}\`（8-14 行精炼代码）
             * \`{"kind": "table", "title": "...", "highlight_col": 1, "columns": [...], "rows": [[...]]}\`
             * \`{"kind": "flowchart", "title": "...", "direction": "TB", "nodes": [{"id": "n1", "label": "..."}], "edges": [{"from": "n1", "to": "n2", "label": "..."}]}\`
           - \`fallback_right\`：右侧结构化备选面板（同样仅使用 \`bullets\` / \`table\` / \`code\` / \`flowchart\` 四种之一，严禁使用 \`mindmap\`；当该页为 \`structured_panel\` 或未启用文生图时直接展示在右栏）。
           - \`visual_mode\`：保持大纲设定的 \`"illustration"\` / \`"arch_diagram"\` / \`"structured_panel"\`。
           - \`illustration_prompt\`：若 \`visual_mode == "illustration"\`，给出 1-2 句精细英文画面提示词（纯视觉构图与隐喻物体，严禁文字与字母）及一句简短中文图注 \`illustration_caption\`。
           - \`notes\`：2-3 句贴合「{{ narration_style }}」的精炼讲师口播逐字稿。
           - \`takeaway\`：页面底部一句凝练的工程结论。

        已确认大纲 JSON：
        {{ outline | json_encode() }}

        参考检索资料：
        {{ search_notes | default(value="（已启用离线领域知识模式）") | truncate(length=10000) }}

  - id: expand_slides
    type: llm
    dependencies: ["expand_prompt"]
    input_mapping:
      prompt: "{{ nodes.expand_prompt.outputs.output }}"
    parameters:
      max_tokens: 8192
      system: "你是资深演示文稿内容主笔，严格输出符合 SlideDeck 扩写 Schema 的合法 JSON。"
      json_schema:
        type: object
        required: [title, slides]
        properties:
          title: { type: string }
          subtitle: { type: string }
          author: { type: string }
          slides:
            type: array
            minItems: 3
            maxItems: 10
            items:
              type: object
              required: [title, left, fallback_right, visual_mode, notes, takeaway]
              properties:
                section: { type: string }
                title: { type: string }
                subtitle: { type: string }
                left:
                  type: object
                  required: [kind]
                  properties:
                    kind:
                      type: string
                      enum: [bullets, code, table, flowchart]
                    title: { type: string }
                    grid: { type: boolean }
                    items:
                      type: array
                      items:
                        type: object
                        required: [heading, body]
                        properties:
                          badge: { type: string }
                          heading: { type: string }
                          body: { type: string }
                          tone: { type: string }
                    language: { type: string }
                    code: { type: string }
                    callouts:
                      type: array
                      items:
                        type: object
                        required: [line, text]
                        properties:
                          line: { type: integer }
                          text: { type: string }
                          tone: { type: string }
                    highlight_col: { type: integer }
                    columns:
                      type: array
                      items: { type: string }
                    rows:
                      type: array
                      items:
                        type: array
                        items: { type: string }
                    direction: { type: string }
                    nodes:
                      type: array
                      items:
                        type: object
                        required: [id, label]
                        properties:
                          id: { type: string }
                          label: { type: string }
                          sublabel: { type: string }
                          tone: { type: string }
                    edges:
                      type: array
                      items:
                        type: object
                        required: [from, to]
                        properties:
                          from: { type: string }
                          to: { type: string }
                          label: { type: string }
                          dashed: { type: boolean }
                fallback_right:
                  type: object
                  required: [kind]
                  properties:
                    kind:
                      type: string
                      enum: [bullets, code, table, flowchart]
                    title: { type: string }
                    grid: { type: boolean }
                    items:
                      type: array
                      items:
                        type: object
                        required: [heading, body]
                        properties:
                          badge: { type: string }
                          heading: { type: string }
                          body: { type: string }
                          tone: { type: string }
                    language: { type: string }
                    code: { type: string }
                    callouts:
                      type: array
                      items:
                        type: object
                        required: [line, text]
                        properties:
                          line: { type: integer }
                          text: { type: string }
                          tone: { type: string }
                    highlight_col: { type: integer }
                    columns:
                      type: array
                      items: { type: string }
                    rows:
                      type: array
                      items:
                        type: array
                        items: { type: string }
                    direction: { type: string }
                    nodes:
                      type: array
                      items:
                        type: object
                        required: [id, label]
                        properties:
                          id: { type: string }
                          label: { type: string }
                          sublabel: { type: string }
                          tone: { type: string }
                    edges:
                      type: array
                      items:
                        type: object
                        required: [from, to]
                        properties:
                          from: { type: string }
                          to: { type: string }
                          label: { type: string }
                          dashed: { type: boolean }
                visual_mode:
                  type: string
                  enum: [illustration, arch_diagram, structured_panel]
                illustration_prompt: { type: string }
                illustration_caption: { type: string }
                takeaway: { type: string }
                notes: { type: string }

  - id: illustrate
    type: illustration
    dependencies: ["expand_slides", "arch_figure", "options"]
    input_mapping:
      deck: "{{ nodes.expand_slides.outputs.output }}"
      arch_image: "{{ nodes.arch_figure.outputs.arch_figure_svg }}"
      model: "{{ nodes.options.outputs.image_model }}"
      size: "{{ nodes.options.outputs.image_size }}"
      style: "{{ nodes.options.outputs.visual_style }}"
      theme: "{{ nodes.options.outputs.slide_theme }}"
      enabled: "{{ nodes.options.outputs.enable_illustrations }}"
    parameters:
      max_images: 4
      max_concurrent: 3
      graceful_fallback: true

  - id: slides_render
    type: render_slides
    dependencies: ["illustrate"]
    input_mapping:
      deck: "{{ nodes.illustrate.outputs.illustrate_deck }}"
    parameters:
      width: 1920
      progressive: false

  - id: deliverable
    type: template
    dependencies: ["illustrate", "slides_render", "arch_figure"]
    input_mapping:
      deck: "{{ nodes.illustrate.outputs.illustrate_deck }}"
      painted: "{{ nodes.illustrate.outputs.illustrate_images }}"
      html: "{{ nodes.slides_render.outputs.slides_render_html }}"
      arch_svg: "{{ nodes.arch_figure.outputs.arch_figure_svg }}"
      pptx: "{{ nodes.slides_render.outputs.slides_render_pptx }}"
      pdf: "{{ nodes.slides_render.outputs.slides_render_pdf }}"
    parameters:
      output_format: json
      template: |
        {"deliverable": {
          "kind": "slides",
          "title": {{ deck.title | json_encode() }},
          "content": {"slides": {{ deck.slides | json_encode() }}},
          "citations": [],
          "files": [
            {"node_id": "slides_render", "name": {{ html | split(pat="/") | last | json_encode() }}, "mime_type": "text/html", "role": "preview"},
            {"node_id": "arch_figure", "name": {{ arch_svg | split(pat="/") | last | json_encode() }}, "mime_type": "image/svg+xml", "role": "asset"}{% for img in painted %},
            {"node_id": "illustrate", "name": {{ img.path | split(pat="/") | last | json_encode() }}, "mime_type": {{ img.mime_type | default(value="image/png") | json_encode() }}, "role": "asset"}{% endfor %}{% if pptx is defined and pptx %},
            {"node_id": "slides_render", "name": {{ pptx | split(pat="/") | last | json_encode() }}, "mime_type": "application/vnd.openxmlformats-officedocument.presentationml.presentation", "role": "primary"}{% endif %}{% if pdf is defined and pdf %},
            {"node_id": "slides_render", "name": {{ pdf | split(pat="/") | last | json_encode() }}, "mime_type": "application/pdf", "role": "primary"}{% endif %}
          ]
        }}
`,X6=`# Y6.3 — \`infographic\` generator (docs/RFC_CONTENT_STUDIO.md §3).
#
# Source (a URL, fetched with the http node's SSRF filter, pasted text,
# or notebook chunks) → structured facts (llm, json_schema: headline
# numbers, sections of short points, a takeaway) → a fixed Tera SVG
# layout (no model-written markup) → SVG + PNG (render_svg) → one
# \`Deliverable\`. The look lives in the two templates below; edit them,
# not code, to restyle.
#
#   yanshi workflow run examples/generators/infographic.yml \\
#     --input url https://example.org/article --output-dir out
name: infographic
description: "An infographic of a source: headline numbers, key points and a takeaway as SVG and PNG"

inputs:
  url:
    description: "Page to summarise (fetched with SSRF protection); leave empty to use \`text\`"
    default: ""
  text:
    description: "Source text, used when \`url\` is empty"
    default: ""
  focus:
    description: "What the infographic should show"
    default: "the key numbers and ideas"
  chunks:
    description: "Notebook passages to work from instead of url / text: [{chunk_id, source_id, content}]. Citations then point at these ids."
    default: []

nodes:
  - id: options
    type: template
    parameters:
      output_format: json
      template: '{"has_url": {{ url != "" }}}'

  # \`url\` (the workflow input) reaches the node directly.
  - id: fetch
    type: http
    dependencies: ["options"]
    run_if: "{{ nodes.options.outputs.has_url }}"

  - id: source
    type: template
    input_mapping:
      page: "{{ nodes.fetch.outputs.body }}"
    parameters:
      template: "{% if chunks | length > 0 %}{% for c in chunks %}[{{ c.chunk_id }}] {{ c.content }}\\n\\n{% endfor %}{% elif page %}{{ page | striptags | truncate(length=24000) }}{% else %}{{ text | truncate(length=24000) }}{% endif %}"

  - id: prompt
    type: template
    dependencies: ["source"]
    input_mapping:
      source_text: "{{ nodes.source.outputs.output }}"
    parameters:
      template: |
        Turn the source below into an infographic about {{ focus }}.
        - \`title\`: at most eight words; \`subtitle\`: one short sentence.
        - \`stats\`: 2-4 headline figures straight from the source; \`value\`
          is the figure itself (at most 8 characters, e.g. "71%", "2x",
          "1.2 bn"), \`label\` says what it measures in at most six words.
          Use no stats rather than invent numbers.
        - \`sections\`: 2-4, each a \`heading\` of at most five words and 2-4
          \`points\` of at most twelve words.
        - \`takeaway\`: the one thing to remember, one sentence.
        Use only what the source supports. When the source's passages
        carry [ids], give every stat and section the chunk_id it comes
        from.

        SOURCE:
        {{ source_text }}

  - id: draft
    type: llm
    dependencies: ["prompt"]
    input_mapping:
      prompt: "{{ nodes.prompt.outputs.output }}"
    parameters:
      system: "You design clear, factual infographics. Answer with JSON only."
      json_schema:
        type: object
        required: [title, stats, sections, takeaway]
        properties:
          title: { type: string }
          subtitle: { type: string }
          stats:
            type: array
            maxItems: 4
            items:
              type: object
              required: [value, label]
              properties:
                value: { type: string }
                label: { type: string }
                chunk_id: { type: string }
          sections:
            type: array
            minItems: 1
            maxItems: 4
            items:
              type: object
              required: [heading, points]
              properties:
                heading: { type: string }
                points: { type: array, minItems: 1, maxItems: 4, items: { type: string } }
                chunk_id: { type: string }
          takeaway: { type: string }

  # Break every text into lines that fit its box. SVG has no text
  # wrapping, so this is where it happens: by words for spaced scripts,
  # by characters (about half as many per line) for CJK and the like.
  # Overlong text is cut to \`max\` lines ending in "…".
  - id: layout
    type: template
    dependencies: ["draft"]
    input_mapping:
      d: "{{ nodes.draft.outputs.output }}"
    parameters:
      output_format: json
      template: |
        {% macro wrap(text, n, max) %}{% set t = text | trim %}{% set words = t | split(pat=" ") %}{% set tl = t | length %}{% set wl = words | length %}{% set_global lines = [] %}{% if wl > 1 and tl / wl < 12 %}{% set_global cur = "" %}{% for w in words %}{% if w != "" %}{% set cl = cur | length %}{% set ww = w | length %}{% if cur == "" %}{% set_global cur = w %}{% elif cl + 1 + ww <= n %}{% set_global cur = cur ~ " " ~ w %}{% else %}{% set_global lines = lines | concat(with=cur) %}{% set_global cur = w %}{% endif %}{% endif %}{% endfor %}{% if cur != "" %}{% set_global lines = lines | concat(with=cur) %}{% endif %}{% elif tl > 0 %}{% set mf = n * 5 / 9 %}{% set m = mf | int %}{% set chars = t | split(pat="") %}{% set last = chars | length %}{% set last = last - 1 %}{% set chars = chars | slice(start=1, end=last) %}{% set cc = chars | length %}{% set count = cc + m - 1 %}{% set count = count / m %}{% set count = count | int %}{% for i in range(end=count) %}{% set a = i * m %}{% set b = a + m %}{% set piece = chars | slice(start=a, end=b) %}{% set piece = piece | join(sep="") %}{% set_global lines = lines | concat(with=piece) %}{% endfor %}{% endif %}{% set ln = lines | length %}{% if ln > max %}{% set k = max - 1 %}{% set tail = lines | nth(n=k) %}{% set tail = tail ~ "…" %}{% set_global lines = lines | slice(end=k) %}{% set_global lines = lines | concat(with=tail) %}{% endif %}{{ lines | json_encode() }}{% endmacro wrap %}
        {"layout": {
          "title": {{ self::wrap(text=d.title, n=38, max=2) }},
          "subtitle": {{ self::wrap(text=d.subtitle | default(value=""), n=84, max=2) }},
          "stats": [{% for s in d.stats | slice(end=4) %}{% if not loop.first %},{% endif %}{"value": {{ s.value | truncate(length=9) | json_encode() }}, "label": {{ self::wrap(text=s.label, n=24, max=3) }}}{% endfor %}],
          "sections": [{% for sec in d.sections | slice(end=4) %}{% if not loop.first %},{% endif %}{"heading": {{ self::wrap(text=sec.heading, n=56, max=2) }}, "points": [{% for p in sec.points | slice(end=4) %}{% if not loop.first %},{% endif %}{{ self::wrap(text=p, n=78, max=3) }}{% endfor %}]}{% endfor %}],
          "takeaway": {{ self::wrap(text=d.takeaway, n=74, max=3) }}
        }}

  # The fixed design: 1200 px wide, as tall as the content. A first
  # pass sums the height, the second draws with the same steps.
  - id: svg_source
    type: template
    dependencies: ["layout"]
    input_mapping:
      L: "{{ nodes.layout.outputs.layout }}"
    parameters:
      template: |
        {%- set colors = ["#2563eb", "#0d9488", "#d97706", "#7c3aed"] -%}
        {%- set_global label_max = 1 -%}{%- for s in L.stats -%}{%- if s.label | length > label_max -%}{%- set_global label_max = s.label | length -%}{%- endif -%}{%- endfor -%}
        {%- set card_h = 132 + label_max * 26 -%}
        {%- set nt = L.title | length -%}{%- set ns = L.subtitle | length -%}{%- set nk = L.takeaway | length -%}{%- set_global h = 60 + nt * 56 -%}
        {%- if ns > 0 -%}{%- set_global h = h + 8 + ns * 32 -%}{%- endif -%}
        {%- if L.stats | length > 0 -%}{%- set_global h = h + 36 + card_h -%}{%- endif -%}
        {%- for sec in L.sections -%}{%- set nh = sec.heading | length -%}{%- set_global h = h + 48 + nh * 34 -%}{%- for p in sec.points -%}{%- set np = p | length -%}{%- set_global h = h + 10 + np * 30 -%}{%- endfor -%}{%- endfor -%}
        {%- if nk > 0 -%}{%- set_global h = h + 88 + nk * 32 -%}{%- endif -%}
        {%- set_global h = h + 80 -%}
        <svg xmlns="http://www.w3.org/2000/svg" width="1200" height="{{ h }}" viewBox="0 0 1200 {{ h }}" font-family="Helvetica, Arial, 'PingFang SC', 'Noto Sans CJK SC', sans-serif">
        <rect width="1200" height="{{ h }}" fill="#f8fafc"/>
        <rect width="1200" height="10" fill="#2563eb"/>
        {%- set_global y = 60 %}
        {% for line in L.title %}<text x="60" y="{{ y + 46 }}" font-size="46" font-weight="700" fill="#0f172a">{{ line | escape }}</text>
        {% set_global y = y + 56 %}{% endfor %}
        {%- if ns > 0 %}{% set_global y = y + 8 %}{% for line in L.subtitle %}<text x="60" y="{{ y + 24 }}" font-size="22" fill="#475569">{{ line | escape }}</text>
        {% set_global y = y + 32 %}{% endfor %}{% endif %}
        {%- if L.stats | length > 0 %}{% set_global y = y + 36 %}{% set k = L.stats | length %}{% set cw = (1080 - (k - 1) * 24) / k %}
        {%- for s in L.stats %}{% set x = 60 + loop.index0 * (cw + 24) %}{% set c = colors | nth(n=loop.index0 % 4) %}
        <rect x="{{ x }}" y="{{ y }}" width="{{ cw }}" height="{{ card_h }}" rx="16" fill="#ffffff" stroke="#e2e8f0" stroke-width="2"/>
        <text x="{{ x + 24 }}" y="{{ y + 84 }}" font-size="52" font-weight="800" fill="{{ c }}">{{ s.value | escape }}</text>
        {% for line in s.label %}<text x="{{ x + 24 }}" y="{{ y + 124 + loop.index0 * 26 }}" font-size="19" fill="#475569">{{ line | escape }}</text>
        {% endfor %}{% endfor %}{% set_global y = y + card_h %}{% endif %}
        {%- for sec in L.sections %}{% set c = colors | nth(n=loop.index0 % 4) %}{% set_global y = y + 40 %}
        {% set nh = sec.heading | length %}<rect x="60" y="{{ y }}" width="8" height="{{ nh * 34 }}" rx="4" fill="{{ c }}"/>
        {% for line in sec.heading %}<text x="84" y="{{ y + 27 }}" font-size="28" font-weight="700" fill="#0f172a">{{ line | escape }}</text>
        {% set_global y = y + 34 %}{% endfor %}{% set_global y = y + 8 %}
        {%- for p in sec.points %}
        <circle cx="94" cy="{{ y + 15 }}" r="5" fill="{{ c }}"/>
        {% for line in p %}<text x="112" y="{{ y + 22 }}" font-size="22" fill="#334155">{{ line | escape }}</text>
        {% set_global y = y + 30 %}{% endfor %}{% set_global y = y + 10 %}{% endfor %}{% endfor %}
        {%- if nk > 0 %}{% set_global y = y + 40 %}{% set th = 48 + nk * 32 %}
        <rect x="60" y="{{ y }}" width="1080" height="{{ th }}" rx="16" fill="#0f172a"/>
        {% for line in L.takeaway %}<text x="92" y="{{ y + 50 + loop.index0 * 32 }}" font-size="24" font-weight="600" fill="#f8fafc">{{ line | escape }}</text>
        {% endfor %}{% set_global y = y + th %}{% endif %}
        <text x="60" y="{{ h - 32 }}" font-size="16" fill="#94a3b8">{% if url %}Source: {{ url | truncate(length=110) | escape }}{% endif %}</text>
        </svg>

  - id: graphic_svg
    type: render_svg
    dependencies: ["svg_source"]
    input_mapping:
      svg: "{{ nodes.svg_source.outputs.output }}"
    parameters:
      format: svg

  - id: graphic_png
    type: render_svg
    dependencies: ["svg_source"]
    input_mapping:
      svg: "{{ nodes.svg_source.outputs.output }}"
    parameters:
      width: 1600

  - id: deliverable
    type: template
    dependencies: ["graphic_svg", "graphic_png"]
    input_mapping:
      d: "{{ nodes.draft.outputs.output }}"
      svg: "{{ nodes.graphic_svg.outputs.graphic_svg_output }}"
      png: "{{ nodes.graphic_png.outputs.graphic_png_output }}"
    parameters:
      output_format: json
      template: |
        {"deliverable": {
          "kind": "infographic",
          "title": {{ d.title | json_encode() }},
          "content": {{ d | json_encode() }},
          "citations": [{% set_global first = true %}{% for n in d.stats | concat(with=d.sections) %}{% if n.chunk_id %}{% set_global sid = "" %}{% set_global passage = "" %}{% for c in chunks %}{% if c.chunk_id == n.chunk_id %}{% set_global sid = c.source_id %}{% set_global passage = c.content %}{% endif %}{% endfor %}{% if passage %}{% if not first %},{% endif %}{% set_global first = false %}{"source_id": {{ sid | json_encode() }}, "chunk_id": {{ n.chunk_id | json_encode() }}, "quote": {{ passage | truncate(length=300) | json_encode() }}}{% endif %}{% endif %}{% endfor %}],
          "files": [
            {"node_id": "graphic_svg", "name": {{ svg | split(pat="/") | last | json_encode() }}, "mime_type": "image/svg+xml", "role": "primary"},
            {"node_id": "graphic_png", "name": {{ png | split(pat="/") | last | json_encode() }}, "mime_type": "image/png", "role": "preview"}
          ]
        }}
`,F6=`# Y4.2 — \`mindmap\` generator (docs/RFC_CONTENT_STUDIO.md §3, §4.3).
#
# Source (a URL, fetched with the http node's SSRF filter, or pasted
# text) → a labelled tree (llm, json_schema) → mind map as SVG, PNG
# (render_svg) and an interactive markmap page → one \`Deliverable\`.
#
#   yanshi workflow run examples/generators/mindmap.yml \\
#     --input url https://example.org/article --output-dir out
name: mindmap
description: "A mind map of a source: SVG, PNG and an interactive HTML page"

inputs:
  url:
    description: "Page to map (fetched with SSRF protection); leave empty to use \`text\`"
    default: ""
  text:
    description: "Source text, used when \`url\` is empty"
    default: ""
  focus:
    description: "What the map should organise"
    default: "the main ideas and how they relate"
  depth:
    description: "Levels below the root (2-4 reads best)"
    default: 3
  chunks:
    description: "Notebook passages to work from instead of url / text: [{chunk_id, source_id, content}]. Citations then point at these ids."
    default: []

nodes:
  - id: options
    type: template
    parameters:
      output_format: json
      template: '{"has_url": {{ url != "" }}}'

  # \`url\` (the workflow input) reaches the node directly.
  - id: fetch
    type: http
    dependencies: ["options"]
    run_if: "{{ nodes.options.outputs.has_url }}"

  - id: source
    type: template
    input_mapping:
      page: "{{ nodes.fetch.outputs.body }}"
    parameters:
      # With notebook \`chunks\`, every passage is labelled with its id so
      # the model can say where each quote comes from.
      template: "{% if chunks | length > 0 %}{% for c in chunks %}[{{ c.chunk_id }}] {{ c.content }}\\n\\n{% endfor %}{% elif page %}{{ page | striptags | truncate(length=24000) }}{% else %}{{ text | truncate(length=24000) }}{% endif %}"

  - id: tree_prompt
    type: template
    dependencies: ["source"]
    input_mapping:
      source_text: "{{ nodes.source.outputs.output }}"
    parameters:
      template: |
        Build a mind map of the source below, organising {{ focus }}.
        The root is the subject in a few words; go at most {{ depth }}
        levels below it, 3-6 children per node, labels of at most six
        words. Use only what the source supports. When the source's
        passages carry [ids], give every first- and second-level node the
        chunk_id of the passage it summarises.

        SOURCE:
        {{ source_text }}

  - id: tree
    type: llm
    dependencies: ["tree_prompt"]
    input_mapping:
      prompt: "{{ nodes.tree_prompt.outputs.output }}"
    parameters:
      system: "You organise ideas into mind maps. Answer with JSON only."
      json_schema:
        $defs:
          node:
            type: object
            required: [label]
            properties:
              label: { type: string }
              chunk_id: { type: string }
              children: { type: array, items: { $ref: "#/$defs/node" } }
        $ref: "#/$defs/node"

  - id: map_svg
    type: render_mindmap
    dependencies: ["tree"]
    input_mapping:
      tree: "{{ nodes.tree.outputs.output }}"

  - id: map_png
    type: render_svg
    dependencies: ["map_svg"]
    input_mapping:
      svg: "{{ nodes.map_svg.outputs.map_svg_output }}"
    parameters:
      width: 1600

  - id: map_html
    type: render_mindmap
    dependencies: ["tree"]
    input_mapping:
      tree: "{{ nodes.tree.outputs.output }}"
    parameters:
      format: html

  - id: deliverable
    type: template
    dependencies: ["map_svg", "map_png", "map_html"]
    input_mapping:
      map: "{{ nodes.tree.outputs.output }}"
      svg: "{{ nodes.map_svg.outputs.map_svg_output }}"
      png: "{{ nodes.map_png.outputs.map_png_output }}"
      html: "{{ nodes.map_html.outputs.map_html_output }}"
    parameters:
      output_format: json
      template: |
        {"deliverable": {
          "kind": "mindmap",
          "title": {{ map.label | json_encode() }},
          "content": {"tree": {{ map | json_encode() }}},
          "citations": [{% set_global first = true %}{% for b in map.children | default(value=[]) %}{% for n in [b] | concat(with=b.children | default(value=[])) %}{% if n.chunk_id %}{% set_global sid = "" %}{% set_global passage = "" %}{% for c in chunks %}{% if c.chunk_id == n.chunk_id %}{% set_global sid = c.source_id %}{% set_global passage = c.content %}{% endif %}{% endfor %}{% if passage %}{% if not first %},{% endif %}{% set_global first = false %}{"source_id": {{ sid | json_encode() }}, "chunk_id": {{ n.chunk_id | json_encode() }}, "quote": {{ passage | truncate(length=300) | json_encode() }}}{% endif %}{% endif %}{% endfor %}{% endfor %}],
          "files": [
            {"node_id": "map_svg", "name": {{ svg | split(pat="/") | last | json_encode() }}, "mime_type": "image/svg+xml", "role": "primary"},
            {"node_id": "map_png", "name": {{ png | split(pat="/") | last | json_encode() }}, "mime_type": "image/png", "role": "preview"},
            {"node_id": "map_html", "name": {{ html | split(pat="/") | last | json_encode() }}, "mime_type": "text/html", "role": "asset"}
          ]
        }}
`,J6=`# Daily Workflow 4 — \`paper_reader\`: PDF URL / arXiv / Local PDF -> Translation + Core Thesis & Claims + Detailed Experimental Data Extraction (XLSX/CSV) + Method Diagram (D2) + Mindmap + PDF Dossier.
#
#   yanshi workflow run examples/generators/paper_reader.yml \\
#     --input source "https://arxiv.org/abs/1706.03762" --output-dir out
name: paper_reader
description: "输入论文 PDF 文件、arXiv 链接或 PDF URL，完成核心章节双语精译、核心思想与论点提炼、详细实验数据表格提取（XLSX/CSV）、算法架构图、思维导图与精读报告 PDF"

inputs:
  source:
    description: "论文 arXiv 链接、PDF URL 或本地上传 PDF（支持 Studio 一键选择本地 PDF）"
    default: "https://arxiv.org/abs/1706.03762"
  target_lang:
    description: "翻译与精读报告目标语言"
    default: "简体中文"
  max_pages:
    description: "最大解析页数上限（含正文、实验表与关键附录）"
    default: 24
  focus:
    description: "重点研读方向"
    default: "问题动机、核心方法创新、数学/架构机制、消融与主实验数据对比、局限性"
  export_pdf:
    description: "同时导出完整论文精读报告 PDF"
    default: true

nodes:
  - id: extract
    type: pdf_extract
    parameters:
      max_chars: 120000

  - id: context_query
    type: template
    dependencies: ["extract"]
    input_mapping:
      paper_text: "{{ nodes.extract.outputs.output }}"
    parameters:
      template: "{{ paper_text | truncate(length=200) | trim }} paper key contributions benchmarks"

  - id: scholarly_context
    type: web_search
    dependencies: ["context_query"]
    input_mapping:
      query: "{{ nodes.context_query.outputs.output }}"
    parameters:
      max_results: 5

  - id: translation_prompt
    type: template
    dependencies: ["extract"]
    input_mapping:
      paper_text: "{{ nodes.extract.outputs.output }}"
    parameters:
      template: |
        你是一位资深计算机与科学文献双语译者。请针对以下提取自论文原件的文本，面向「{{ target_lang }}」读者完成高质量忠实精译（信、达、专有名词保留英文括注，公式符号保持原样）：
        1. \`paper_title_original\` 与 \`paper_title_translated\`；
        2. \`abstract_bilingual\`：摘要原文与{{ target_lang }}逐段对照精译；
        3. \`section_translations\`：按论文 4-6 个最核心章节顺序（引言、核心方法/模型设计、关键理论/架构细节、结论），输出每一节的高密度精准{{ target_lang }}精译（每节控制在 200-350 字核心译文）与术语批注。

        论文全文提取内容：
        {{ paper_text | truncate(length=52000) }}

  - id: translate_paper
    type: llm
    dependencies: ["translation_prompt"]
    input_mapping:
      prompt: "{{ nodes.translation_prompt.outputs.output }}"
    parameters:
      max_tokens: 8192
      system: "你是严谨的学术论文翻译专家，术语准确、逻辑忠实于原文，仅输出合法 JSON。"
      json_schema:
        type: object
        required: [paper_title_original, paper_title_translated, abstract_bilingual, section_translations]
        properties:
          paper_title_original: { type: string }
          paper_title_translated: { type: string }
          authors_and_venue: { type: string }
          abstract_bilingual: { type: string }
          section_translations:
            type: array
            minItems: 2
            maxItems: 7
            items:
              type: object
              required: [section_heading, translated_markdown, key_terms]
              properties:
                section_heading: { type: string }
                translated_markdown: { type: string }
                key_terms:
                  type: array
                  items: { type: string }

  - id: thesis_prompt
    type: template
    dependencies: ["extract", "scholarly_context"]
    input_mapping:
      paper_text: "{{ nodes.extract.outputs.output }}"
      web_ctx: "{{ nodes.scholarly_context.outputs.markdown }}"
    parameters:
      template: |
        请以顶会审稿人与领域研究者的视角，围绕「{{ focus }}」深度精读以下论文，并用「{{ target_lang }}」提炼：
        1. \`one_sentence_insight\`：一句话直击论文最本质的核心思想突破；
        2. \`problem_and_bottleneck\`：既有方法面临的根本瓶颈与本文切入点；
        3. \`core_claims\`：论文提出的 3-6 条核心论点，每条论点给出对应的理论推导或实验证据支撑（注明章节/图表出处）；
        4. \`method_mechanics\`：算法/架构关键机制拆解（含关键公式含义与复杂度权衡）；
        5. \`limitations_and_future\`：边界条件、局限性与后续演进脉络；
        6. \`method_d2_brief\`：用于 D2 绘制「论文核心算法/模型数据流架构图」的精确结构说明；
        7. \`mindmap_tree\`：论文全景知识思维导图树（根节点为论文简称，含研究动机、核心思想、关键方法、实验证据、启发与局限等分支，深度不超过 3 层）。

        论文正文：
        {{ paper_text | truncate(length=52000) }}

        学术背景补充：
        {{ web_ctx | truncate(length=6000) }}

  - id: analyze_thesis
    type: llm
    dependencies: ["thesis_prompt"]
    input_mapping:
      prompt: "{{ nodes.thesis_prompt.outputs.output }}"
    parameters:
      max_tokens: 8192
      system: "你是严谨的顶会审稿人，一切论点与数据必须严格忠于论文原文，严禁编造数值，仅输出合法 JSON。"
      json_schema:
        type: object
        required:
          - one_sentence_insight
          - problem_and_bottleneck
          - core_claims
          - method_mechanics
          - limitations_and_future
          - method_d2_brief
          - mindmap_tree
        properties:
          one_sentence_insight: { type: string }
          problem_and_bottleneck: { type: string }
          core_claims:
            type: array
            minItems: 2
            maxItems: 8
            items:
              type: object
              required: [claim, evidence_in_paper, source_section]
              properties:
                claim: { type: string }
                evidence_in_paper: { type: string }
                source_section: { type: string }
          method_mechanics: { type: string }
          limitations_and_future: { type: string }
          method_d2_brief: { type: string }
          mindmap_tree:
            type: object
            required: [label, children]
            properties:
              label: { type: string }
              children:
                type: array
                items:
                  type: object
                  required: [label]
                  properties:
                    label: { type: string }
                    children:
                      type: array
                      items:
                        type: object
                        required: [label]
                        properties:
                          label: { type: string }
                          children:
                            type: array
                            items:
                              type: object
                              required: [label]
                              properties:
                                label: { type: string }

  - id: experiments_prompt
    type: template
    dependencies: ["extract"]
    input_mapping:
      paper_text: "{{ nodes.extract.outputs.output }}"
    parameters:
      template: |
        请从以下论文全文中提取最具代表性的 8-18 行关键实验设置与定量对比数据（涵盖主实验、基线对比、消融实验 Ablation Study、吞吐/参数量/开销对比）：
        1. \`experimental_setup\`：数据集、评价指标、训练/推理硬件与关键超参数总结；
        2. \`ablation_findings\`：关键消融实验发现与变量贡献度解读；
        3. \`metrics_rows\`：结构化实验数据行数组（8-18 行核心数据），每行包含统一字段：
           - \`table_or_figure\`（原论文图表编号，如 "Table 1" / "Table 3"）
           - \`task_or_dataset\`（任务或数据集名称）
           - \`model_or_variant\`（对比基线或本文模型变体）
           - \`metric_name\`（指标名称，如 BLEU / Accuracy / Latency / Params）
           - \`metric_value\`（论文原始数值字符串，原样保留）
           - \`delta_vs_baseline\`（相对最强基线或基准变体的提升/变化说明）
           - \`supports_claim\`（该组数据支撑的核心论点）

        论文全文：
        {{ paper_text | truncate(length=56000) }}

  - id: extract_experiments
    type: llm
    dependencies: ["experiments_prompt"]
    input_mapping:
      prompt: "{{ nodes.experiments_prompt.outputs.output }}"
    parameters:
      max_tokens: 8192
      system: "你是定量科学数据分析专家，严格提取论文真实实验表格与正文报告的原始数据，绝不虚构指标，仅输出合法 JSON。"
      json_schema:
        type: object
        required: [experimental_setup, ablation_findings, metrics_rows]
        properties:
          experimental_setup: { type: string }
          ablation_findings: { type: string }
          metrics_rows:
            type: array
            minItems: 1
            maxItems: 18
            items:
              type: object
              required:
                - table_or_figure
                - task_or_dataset
                - model_or_variant
                - metric_name
                - metric_value
                - delta_vs_baseline
                - supports_claim
              properties:
                table_or_figure: { type: string }
                task_or_dataset: { type: string }
                model_or_variant: { type: string }
                metric_name: { type: string }
                metric_value: { type: string }
                delta_vs_baseline: { type: string }
                supports_claim: { type: string }

  - id: exp_rows_json
    type: template
    dependencies: ["extract_experiments"]
    input_mapping:
      exp: "{{ nodes.extract_experiments.outputs.output }}"
    parameters:
      output_format: json
      template: "{{ exp.metrics_rows | json_encode() }}"

  - id: experiments_xlsx
    type: render_table
    dependencies: ["exp_rows_json"]
    input_mapping:
      rows: "{{ nodes.exp_rows_json.outputs.output }}"
    parameters:
      format: xlsx
      columns:
        - table_or_figure
        - task_or_dataset
        - model_or_variant
        - metric_name
        - metric_value
        - delta_vs_baseline
        - supports_claim

  - id: experiments_csv
    type: render_table
    dependencies: ["exp_rows_json"]
    input_mapping:
      rows: "{{ nodes.exp_rows_json.outputs.output }}"
    parameters:
      format: csv
      columns:
        - table_or_figure
        - task_or_dataset
        - model_or_variant
        - metric_name
        - metric_value
        - delta_vs_baseline
        - supports_claim

  - id: method_fig_desc
    type: template
    dependencies: ["analyze_thesis", "translate_paper"]
    input_mapping:
      thesis: "{{ nodes.analyze_thesis.outputs.output }}"
      trans: "{{ nodes.translate_paper.outputs.output }}"
    parameters:
      template: |
        为论文《{{ trans.paper_title_original }}》绘制一张清晰展示其核心模型/算法数据流机制的 D2 架构图（direction: right，8-14 个关键模块节点，标注张量/数据流转换关系）：
        {{ thesis.method_d2_brief }}

  - id: method_diagram
    type: figure
    dependencies: ["method_fig_desc"]
    input_mapping:
      description: "{{ nodes.method_fig_desc.outputs.output }}"
    parameters:
      language: d2
      png: true
      png_width: 1800
      max_attempts: 4

  - id: mindmap_tree_json
    type: template
    dependencies: ["analyze_thesis"]
    input_mapping:
      thesis: "{{ nodes.analyze_thesis.outputs.output }}"
    parameters:
      output_format: json
      template: '{"tree": {{ thesis.mindmap_tree | json_encode() }}}'

  - id: mindmap_svg
    type: render_mindmap
    dependencies: ["mindmap_tree_json"]
    input_mapping:
      tree: "{{ nodes.mindmap_tree_json.outputs.tree }}"

  - id: mindmap_png
    type: render_svg
    dependencies: ["mindmap_svg"]
    input_mapping:
      svg: "{{ nodes.mindmap_svg.outputs.mindmap_svg_output }}"
    parameters:
      width: 1600

  - id: mindmap_html
    type: render_mindmap
    dependencies: ["mindmap_tree_json"]
    input_mapping:
      tree: "{{ nodes.mindmap_tree_json.outputs.tree }}"
    parameters:
      format: html

  - id: assemble_dossier
    type: template
    dependencies:
      - extract
      - translate_paper
      - analyze_thesis
      - extract_experiments
      - method_diagram
      - mindmap_png
    input_mapping:
      pages: "{{ nodes.extract.outputs.page_count }}"
      trans: "{{ nodes.translate_paper.outputs.output }}"
      thesis: "{{ nodes.analyze_thesis.outputs.output }}"
      exp: "{{ nodes.extract_experiments.outputs.output }}"
      method_png: "{{ nodes.method_diagram.outputs.method_diagram_png }}"
      mm_png: "{{ nodes.mindmap_png.outputs.mindmap_png_output }}"
    parameters:
      template: |
        # {{ trans.paper_title_translated }}

        > **原文标题**：*{{ trans.paper_title_original }}*
        {% if trans.authors_and_venue %}> **作者与出处**：{{ trans.authors_and_venue }}{% endif %}
        > **解析页数**：共 {{ pages }} 页

        ## 1. 一句话核心思想与知识思维导图

        > **核心突破**：{{ thesis.one_sentence_insight }}

        ![论文全景思维导图]({{ mm_png }})

        ## 2. 研究问题与核心论点链条

        ### 2.1 既有瓶颈与问题定义

        {{ thesis.problem_and_bottleneck }}

        ### 2.2 核心论点与原文证据映射

        | 序号 | 核心论点 (Claim) | 论文实验/理论证据支撑 | 原文位置 |
        | --- | --- | --- | --- |
        {% for c in thesis.core_claims %}| {{ loop.index }} | **{{ c.claim }}** | {{ c.evidence_in_paper }} | \`{{ c.source_section }}\` |
        {% endfor %}

        ## 3. 核心算法与架构机制深度拆解

        ![核心方法架构图]({{ method_png }})

        {{ thesis.method_mechanics }}

        ## 4. 详细实验设置与定量数据提取

        ### 4.1 实验设置与基准说明

        {{ exp.experimental_setup }}

        ### 4.2 定量实验结果对比总表

        | 来源图表 | 任务 / 数据集 | 模型 / 变体 | 指标 | 数值 | 相对基线变化 | 支撑论点 |
        | --- | --- | --- | --- | --- | --- | --- |
        {% for r in exp.metrics_rows %}| \`{{ r.table_or_figure }}\` | {{ r.task_or_dataset }} | \`{{ r.model_or_variant }}\` | {{ r.metric_name }} | **{{ r.metric_value }}** | {{ r.delta_vs_baseline }} | {{ r.supports_claim }} |
        {% endfor %}

        ### 4.3 消融实验与关键规律解读

        {{ exp.ablation_findings }}

        ## 5. 摘要双语对照与核心章节精译

        ### 5.1 摘要双语对照

        {{ trans.abstract_bilingual }}

        {% for sec in trans.section_translations %}
        ### 5.{{ loop.index + 1 }} {{ sec.section_heading }}

        {{ sec.translated_markdown }}
        {% if sec.key_terms | length > 0 %}
        - **关键术语对照**：{{ sec.key_terms | join(sep=" · ") }}
        {% endif %}
        {% endfor %}

        ## 6. 局限性与后续研究启发

        {{ thesis.limitations_and_future }}

  - id: dossier_md
    type: render_markdown
    dependencies: ["assemble_dossier"]
    input_mapping:
      markdown: "{{ nodes.assemble_dossier.outputs.output }}"
    parameters:
      format: md

  - id: dossier_html
    type: render_markdown
    dependencies: ["assemble_dossier"]
    input_mapping:
      markdown: "{{ nodes.assemble_dossier.outputs.output }}"

  - id: pdf_flag
    type: template
    parameters:
      output_format: json
      template: '{"want_pdf": {{ export_pdf }}}'

  - id: dossier_pdf
    type: render_document
    dependencies: ["assemble_dossier", "pdf_flag"]
    run_if: "{{ nodes.pdf_flag.outputs.want_pdf }}"
    input_mapping:
      markdown: "{{ nodes.assemble_dossier.outputs.output }}"
    parameters:
      format: pdf
      toc: true

  - id: deliverable
    type: template
    dependencies:
      - translate_paper
      - analyze_thesis
      - extract_experiments
      - experiments_xlsx
      - experiments_csv
      - method_diagram
      - mindmap_svg
      - mindmap_html
      - dossier_md
      - dossier_html
      - dossier_pdf
    input_mapping:
      trans: "{{ nodes.translate_paper.outputs.output }}"
      thesis: "{{ nodes.analyze_thesis.outputs.output }}"
      exp: "{{ nodes.extract_experiments.outputs.output }}"
      xlsx: "{{ nodes.experiments_xlsx.outputs.experiments_xlsx_output }}"
      csv: "{{ nodes.experiments_csv.outputs.experiments_csv_output }}"
      fig_svg: "{{ nodes.method_diagram.outputs.method_diagram_svg }}"
      mm_svg: "{{ nodes.mindmap_svg.outputs.mindmap_svg_output }}"
      mm_html: "{{ nodes.mindmap_html.outputs.mindmap_html_output }}"
      md: "{{ nodes.dossier_md.outputs.dossier_md_output }}"
      html: "{{ nodes.dossier_html.outputs.dossier_html_output }}"
      pdf: "{{ nodes.dossier_pdf.outputs.dossier_pdf_output }}"
    parameters:
      output_format: json
      template: |
        {"deliverable": {
          "kind": "paper_reader",
          "title": {{ trans.paper_title_translated | json_encode() }},
          "content": {"thesis": {{ thesis | json_encode() }}, "experiments": {{ exp | json_encode() }}},
          "citations": [],
          "files": [
            {"node_id": "dossier_html", "name": {{ html | split(pat="/") | last | json_encode() }}, "mime_type": "text/html", "role": "preview"},
            {"node_id": "dossier_md", "name": {{ md | split(pat="/") | last | json_encode() }}, "mime_type": "text/markdown", "role": "asset"},
            {"node_id": "experiments_xlsx", "name": {{ xlsx | split(pat="/") | last | json_encode() }}, "mime_type": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", "role": "primary"},
            {"node_id": "experiments_csv", "name": {{ csv | split(pat="/") | last | json_encode() }}, "mime_type": "text/csv", "role": "asset"},
            {"node_id": "method_diagram", "name": {{ fig_svg | split(pat="/") | last | json_encode() }}, "mime_type": "image/svg+xml", "role": "asset"},
            {"node_id": "mindmap_svg", "name": {{ mm_svg | split(pat="/") | last | json_encode() }}, "mime_type": "image/svg+xml", "role": "asset"},
            {"node_id": "mindmap_html", "name": {{ mm_html | split(pat="/") | last | json_encode() }}, "mime_type": "text/html", "role": "preview"}{% if pdf %},
            {"node_id": "dossier_pdf", "name": {{ pdf | split(pat="/") | last | json_encode() }}, "mime_type": "application/pdf", "role": "primary"}{% endif %}
          ]
        }}
`,Q6=`# Y6.2 — \`quiz\` generator (docs/RFC_CONTENT_STUDIO.md §3).
#
# Source (a URL, fetched with the http node's SSRF filter, pasted text,
# or notebook chunks) → multiple-choice questions (llm, json_schema) →
# one self-contained HTML page that checks answers (render_study) → one
# \`Deliverable\` whose content carries the questions as JSON.
#
#   yanshi workflow run examples/generators/quiz.yml \\
#     --input url https://example.org/article --output-dir out
name: quiz
description: "A multiple-choice quiz on a source: JSON questions and a self-checking HTML page"

inputs:
  url:
    description: "Page to study (fetched with SSRF protection); leave empty to use \`text\`"
    default: ""
  text:
    description: "Source text, used when \`url\` is empty"
    default: ""
  focus:
    description: "What the quiz should test"
    default: "the key facts and ideas"
  count:
    description: "How many questions (3-20)"
    default: 8
  chunks:
    description: "Notebook passages to work from instead of url / text: [{chunk_id, source_id, content}]. Citations then point at these ids."
    default: []

nodes:
  - id: options
    type: template
    parameters:
      output_format: json
      template: '{"has_url": {{ url != "" }}}'

  # \`url\` (the workflow input) reaches the node directly.
  - id: fetch
    type: http
    dependencies: ["options"]
    run_if: "{{ nodes.options.outputs.has_url }}"

  - id: source
    type: template
    input_mapping:
      page: "{{ nodes.fetch.outputs.body }}"
    parameters:
      template: "{% if chunks | length > 0 %}{% for c in chunks %}[{{ c.chunk_id }}] {{ c.content }}\\n\\n{% endfor %}{% elif page %}{{ page | striptags | truncate(length=24000) }}{% else %}{{ text | truncate(length=24000) }}{% endif %}"

  - id: prompt
    type: template
    dependencies: ["source"]
    input_mapping:
      source_text: "{{ nodes.source.outputs.output }}"
    parameters:
      template: |
        Write {{ count }} multiple-choice questions on the source below,
        testing {{ focus }}. Each has 3-5 \`options\`, exactly one right;
        \`answer\` is the 0-based index of the right option, and
        \`explanation\` says in one sentence why, from the source. Wrong
        options must be plausible. Use only what the source supports.
        When the source's passages carry [ids], give every question the
        chunk_id of the passage it comes from.

        SOURCE:
        {{ source_text }}

  - id: draft
    type: llm
    dependencies: ["prompt"]
    input_mapping:
      prompt: "{{ nodes.prompt.outputs.output }}"
    parameters:
      system: "You write fair, unambiguous quiz questions. Answer with JSON only."
      json_schema:
        type: object
        required: [title, items]
        properties:
          title: { type: string }
          items:
            type: array
            minItems: 1
            items:
              type: object
              required: [question, options, answer]
              properties:
                question: { type: string }
                options: { type: array, minItems: 2, maxItems: 8, items: { type: string } }
                answer: { type: integer, minimum: 0 }
                explanation: { type: string }
                chunk_id: { type: string }

  # Lift the items out of the model's answer for the renderer.
  - id: lifted
    type: template
    dependencies: ["draft"]
    input_mapping:
      draft: "{{ nodes.draft.outputs.output }}"
    parameters:
      output_format: json
      template: '{"title": {{ draft.title | json_encode() }}, "items": {{ draft["items"] | json_encode() }}}'

  - id: page
    type: render_study
    dependencies: ["lifted"]
    input_mapping:
      title: "{{ nodes.lifted.outputs.title }}"
      items: "{{ nodes.lifted.outputs.items }}"
    parameters:
      kind: quiz

  - id: deliverable
    type: template
    dependencies: ["page"]
    input_mapping:
      drafted: "{{ nodes.draft.outputs.output }}"
      html: "{{ nodes.page.outputs.page_output }}"
    parameters:
      output_format: json
      template: |
        {"deliverable": {
          "kind": "quiz",
          "title": {{ drafted.title | json_encode() }},
          "content": {"items": {{ drafted["items"] | json_encode() }}},
          "citations": [{% set_global first = true %}{% for n in drafted["items"] %}{% if n.chunk_id %}{% set_global sid = "" %}{% set_global passage = "" %}{% for c in chunks %}{% if c.chunk_id == n.chunk_id %}{% set_global sid = c.source_id %}{% set_global passage = c.content %}{% endif %}{% endfor %}{% if passage %}{% if not first %},{% endif %}{% set_global first = false %}{"source_id": {{ sid | json_encode() }}, "chunk_id": {{ n.chunk_id | json_encode() }}, "quote": {{ passage | truncate(length=300) | json_encode() }}}{% endif %}{% endif %}{% endfor %}],
          "files": [
            {"node_id": "page", "name": {{ html | split(pat="/") | last | json_encode() }}, "mime_type": "text/html", "role": "primary"}
          ]
        }}
`,W6=`# Daily Workflow 2 — \`repo_architecture\`: GitHub / Source Code Snapshot -> Architecture Summary + System Architecture Diagram (D2) + Module Dependency Graph (D2) + PDF/HTML/Markdown Report.
#
#   yanshi workflow run examples/generators/repo_architecture.yml \\
#     --input source "https://github.com/tokio-rs/axum" --output-dir out
name: repo_architecture
description: "输入 GitHub 仓库链接或受控源码目录，自动扫描目录树、构建配置与内部依赖拓扑，生成编译校验的 D2 系统分层架构图、模块依赖图与深度架构分析报告"

inputs:
  source:
    description: "GitHub 仓库链接（如 https://github.com/owner/repo）或受控本地源码目录"
    default: "https://github.com/tokio-rs/axum"
  git_ref:
    description: "可选 Git 分支、Tag 或 Commit SHA（留空自动取默认主分支）"
    default: ""
  subpath:
    description: "可选子目录路径（大型 Monorepo 可聚焦子目录）"
    default: ""
  focus:
    description: "架构提炼关注重点"
    default: "整体分层架构、核心模块职责边界、数据/请求流转链路与模块间依赖拓扑"
  export_pdf:
    description: "同时导出排版精美的 PDF 架构报告"
    default: true

nodes:
  - id: snapshot
    type: repo_snapshot
    parameters:
      max_files: 28
      max_bytes: 180000

  - id: analysis_prompt
    type: template
    dependencies: ["snapshot"]
    input_mapping:
      repo_md: "{{ nodes.snapshot.outputs.markdown }}"
    parameters:
      template: |
        你是一位首席软件架构师。请基于下方从真实代码仓库提取的目录树、构建清单（Cargo.toml / package.json / go.mod / pyproject.toml 等）、静态扫描到的模块依赖边与关键入口源码，围绕「{{ focus }}」对该项目进行深度逆向架构提炼。

        输出要求：
        1. \`project_title\`：项目名称与一句话架构定位；
        2. \`executive_summary\`：2 段精炼高密度的架构总览（解决的核心问题、设计哲学与整体分层模型）；
        3. \`layers\`：提炼出 3-6 个清晰的架构分层（每层含 \`name\`、\`responsibility\`、\`modules\`）；
        4. \`modules\`：聚焦最重要的 6-12 个核心模块/Crate/Package 职责清单（含 \`name\`、\`path\`、\`role\` 精炼在 60 字以内、\`depends_on\`）；
        5. \`request_or_data_flow\`：核心执行链路或请求生命周期拆解（300 字以内）；
        6. \`system_arch_d2_brief\`：用于交给 D2 绘图引擎生成「系统分层与运行时架构图」的精确结构描述（明确各层嵌套容器、核心子系统及关键交互流向，严格忠于真实模块名）；
        7. \`module_dep_d2_brief\`：用于交给 D2 绘图引擎生成「项目模块依赖有向拓扑图（DAG）」的精确结构描述（列出真实 crate / package / 模块节点及 \`A -> B\` 依赖关系，严禁臆造不存在的包名）。

        代码仓库快照：
        {{ repo_md | truncate(length=48000) }}

  - id: analyze_repo
    type: llm
    dependencies: ["analysis_prompt"]
    input_mapping:
      prompt: "{{ nodes.analysis_prompt.outputs.output }}"
    parameters:
      max_tokens: 8192
      system: "你是严谨的系统架构分析师，所有模块名与依赖关系必须严格基于仓库真实清单与源码，仅输出合法 JSON。"
      json_schema:
        type: object
        required:
          - project_title
          - executive_summary
          - layers
          - modules
          - request_or_data_flow
          - system_arch_d2_brief
          - module_dep_d2_brief
        properties:
          project_title: { type: string }
          executive_summary: { type: string }
          layers:
            type: array
            minItems: 1
            maxItems: 8
            items:
              type: object
              required: [name, responsibility, modules]
              properties:
                name: { type: string }
                responsibility: { type: string }
                modules:
                  type: array
                  items: { type: string }
          modules:
            type: array
            minItems: 1
            maxItems: 12
            items:
              type: object
              required: [name, path, role, depends_on]
              properties:
                name: { type: string }
                path: { type: string }
                role: { type: string }
                depends_on:
                  type: array
                  items: { type: string }
          request_or_data_flow: { type: string }
          architectural_highlights:
            type: array
            items: { type: string }
          system_arch_d2_brief: { type: string }
          module_dep_d2_brief: { type: string }

  - id: arch_fig_prompt
    type: template
    dependencies: ["analyze_repo"]
    input_mapping:
      spec: "{{ nodes.analyze_repo.outputs.output }}"
    parameters:
      template: |
        为项目「{{ spec.project_title }}」绘制一张高可读性的 D2 系统分层架构图（建议 \`direction: down\`，每个架构层使用 D2 容器 \`{ label: "..." ... }\` 包裹对应子模块，并用带简短动词标签的有向边连接核心数据/调用链路）：

        {{ spec.system_arch_d2_brief }}

  - id: system_arch_diagram
    type: figure
    dependencies: ["arch_fig_prompt"]
    input_mapping:
      description: "{{ nodes.arch_fig_prompt.outputs.output }}"
    parameters:
      language: d2
      png: true
      png_width: 1800
      max_attempts: 4

  - id: dep_fig_prompt
    type: template
    dependencies: ["analyze_repo", "snapshot"]
    input_mapping:
      spec: "{{ nodes.analyze_repo.outputs.output }}"
      hints: "{{ nodes.snapshot.outputs.dependency_hints }}"
    parameters:
      template: |
        为项目「{{ spec.project_title }}」绘制一张精确的 D2 模块/包依赖拓扑图（建议 \`direction: right\` 或 \`direction: down\`，按真实模块分组，使用有向边 \`caller -> dependency\` 表达依赖方向，保持节点命名与真实包名/目录名一致）：

        架构师依赖说明：
        {{ spec.module_dep_d2_brief }}

        静态扫描到的真实依赖边参考：
        {{ hints | json_encode() }}

  - id: module_dep_diagram
    type: figure
    dependencies: ["dep_fig_prompt"]
    input_mapping:
      description: "{{ nodes.dep_fig_prompt.outputs.output }}"
    parameters:
      language: d2
      png: true
      png_width: 1800
      max_attempts: 4

  - id: assemble_report
    type: template
    dependencies: ["analyze_repo", "system_arch_diagram", "module_dep_diagram"]
    input_mapping:
      spec: "{{ nodes.analyze_repo.outputs.output }}"
      arch_png: "{{ nodes.system_arch_diagram.outputs.system_arch_diagram_png }}"
      dep_png: "{{ nodes.module_dep_diagram.outputs.module_dep_diagram_png }}"
    parameters:
      template: |
        # {{ spec.project_title }} — 架构与模块依赖深度解析

        > **源码来源**：\`{{ source }}\`{% if git_ref != "" %} (\`{{ git_ref }}\`){% endif %}{% if subpath != "" %} · 子路径 \`{{ subpath }}\`{% endif %}

        ## 1. 架构总览

        {{ spec.executive_summary }}

        ![系统分层架构图]({{ arch_png }})

        ## 2. 分层架构模型

        {% for layer in spec.layers %}
        ### {{ loop.index }}. {{ layer.name }}
        - **核心职责**：{{ layer.responsibility }}
        - **包含模块**：{% for m in layer.modules %}\`{{ m }}\`{% if not loop.last %}、{% endif %}{% endfor %}
        {% endfor %}

        ## 3. 模块依赖拓扑与职责矩阵

        ![模块依赖拓扑图]({{ dep_png }})

        | 模块名称 | 源码路径 | 核心职责 | 直接依赖模块 |
        | --- | --- | --- | --- |
        {% for m in spec.modules %}| \`{{ m.name }}\` | \`{{ m.path }}\` | {{ m.role }} | {% if m.depends_on | length > 0 %}{% for d in m.depends_on %}\`{{ d }}\`{% if not loop.last %}, {% endif %}{% endfor %}{% else %}—{% endif %} |
        {% endfor %}

        ## 4. 核心执行与数据流链路

        {{ spec.request_or_data_flow }}

        {% if spec.architectural_highlights and spec.architectural_highlights | length > 0 %}
        ## 5. 关键工程设计亮点

        {% for h in spec.architectural_highlights %}- {{ h }}
        {% endfor %}
        {% endif %}

  - id: report_md
    type: render_markdown
    dependencies: ["assemble_report"]
    input_mapping:
      markdown: "{{ nodes.assemble_report.outputs.output }}"
    parameters:
      format: md

  - id: report_html
    type: render_markdown
    dependencies: ["assemble_report"]
    input_mapping:
      markdown: "{{ nodes.assemble_report.outputs.output }}"

  - id: pdf_flag
    type: template
    parameters:
      output_format: json
      template: '{"want_pdf": {{ export_pdf }}}'

  - id: report_pdf
    type: render_document
    dependencies: ["assemble_report", "pdf_flag"]
    run_if: "{{ nodes.pdf_flag.outputs.want_pdf }}"
    input_mapping:
      markdown: "{{ nodes.assemble_report.outputs.output }}"
    parameters:
      format: pdf
      toc: true

  - id: deliverable
    type: template
    dependencies: ["analyze_repo", "system_arch_diagram", "module_dep_diagram", "report_md", "report_html", "report_pdf"]
    input_mapping:
      spec: "{{ nodes.analyze_repo.outputs.output }}"
      arch_svg: "{{ nodes.system_arch_diagram.outputs.system_arch_diagram_svg }}"
      arch_png: "{{ nodes.system_arch_diagram.outputs.system_arch_diagram_png }}"
      dep_svg: "{{ nodes.module_dep_diagram.outputs.module_dep_diagram_svg }}"
      dep_png: "{{ nodes.module_dep_diagram.outputs.module_dep_diagram_png }}"
      md: "{{ nodes.report_md.outputs.report_md_output }}"
      html: "{{ nodes.report_html.outputs.report_html_output }}"
      pdf: "{{ nodes.report_pdf.outputs.report_pdf_output }}"
    parameters:
      output_format: json
      template: |
        {"deliverable": {
          "kind": "repo_architecture",
          "title": {{ spec.project_title | json_encode() }},
          "content": {{ spec | json_encode() }},
          "citations": [],
          "files": [
            {"node_id": "system_arch_diagram", "name": {{ arch_svg | split(pat="/") | last | json_encode() }}, "mime_type": "image/svg+xml", "role": "primary"},
            {"node_id": "system_arch_diagram", "name": {{ arch_png | split(pat="/") | last | json_encode() }}, "mime_type": "image/png", "role": "preview"},
            {"node_id": "module_dep_diagram", "name": {{ dep_svg | split(pat="/") | last | json_encode() }}, "mime_type": "image/svg+xml", "role": "primary"},
            {"node_id": "module_dep_diagram", "name": {{ dep_png | split(pat="/") | last | json_encode() }}, "mime_type": "image/png", "role": "preview"},
            {"node_id": "report_md", "name": {{ md | split(pat="/") | last | json_encode() }}, "mime_type": "text/markdown", "role": "asset"},
            {"node_id": "report_html", "name": {{ html | split(pat="/") | last | json_encode() }}, "mime_type": "text/html", "role": "asset"}{% if pdf %},
            {"node_id": "report_pdf", "name": {{ pdf | split(pat="/") | last | json_encode() }}, "mime_type": "application/pdf", "role": "primary"}{% endif %}
          ]
        }}
`,e5=`# Y4.1 — \`report\` generator (docs/RFC_CONTENT_STUDIO.md §3, §4.3).
#
# Source (a URL, fetched with the http node's SSRF filter, or pasted
# text) → outline (llm, json_schema) → sections written in parallel (Map,
# each with the quotes it relies on) → Markdown with footnoted quotes →
# .md + .html (+ .pdf via pandoc / typst) → one \`Deliverable\` JSON.
#
#   yanshi workflow run examples/generators/report.yml \\
#     --input url=https://example.org/article --output-dir out
#   yanshi workflow run examples/generators/report.yml \\
#     --input text="$(cat notes.txt)" --input pdf=false --model <model>
name: report
description: "A sourced report: outline, sections, footnoted quotes; .md, .html and .pdf"

inputs:
  url:
    description: "Page to report on (fetched with SSRF protection); leave empty to use \`text\`"
    default: ""
  text:
    description: "Source text, used when \`url\` is empty"
    default: ""
  focus:
    description: "What the report should concentrate on"
    default: "the main points, with their supporting evidence"
  sections:
    description: "How many sections to write"
    default: 4
  pdf:
    description: "Also render a PDF (needs pandoc + typst; see \`yanshi doctor\`)"
    default: true
  chunks:
    description: "Notebook passages to work from instead of url / text: [{chunk_id, source_id, content}]. Citations then point at these ids."
    default: []

nodes:
  # run_if can read node outputs only, so the switches go through a node.
  - id: options
    type: template
    parameters:
      output_format: json
      template: '{"has_url": {{ url != "" }}, "want_pdf": {{ pdf }}}'

  # \`url\` (the workflow input) reaches the node directly.
  - id: fetch
    type: http
    dependencies: ["options"]
    run_if: "{{ nodes.options.outputs.has_url }}"

  # The loader: HTML tags stripped, or the pasted text as is. \`fetch\` is a
  # soft reference: when it was skipped, \`page\` is simply absent.
  - id: source
    type: template
    input_mapping:
      page: "{{ nodes.fetch.outputs.body }}"
    parameters:
      # With notebook \`chunks\`, every passage is labelled with its id so
      # the model can say where each quote comes from.
      template: "{% if chunks | length > 0 %}{% for c in chunks %}[{{ c.chunk_id }}] {{ c.content }}\\n\\n{% endfor %}{% elif page %}{{ page | striptags | truncate(length=24000) }}{% else %}{{ text | truncate(length=24000) }}{% endif %}"

  - id: outline_prompt
    type: template
    dependencies: ["source"]
    input_mapping:
      source_text: "{{ nodes.source.outputs.output }}"
    parameters:
      template: |
        Plan a report on the source below, concentrating on: {{ focus }}.
        Give it a title and exactly {{ sections }} sections, each with a
        heading and a one-sentence brief of what it covers. Use only what
        the source supports.

        SOURCE:
        {{ source_text }}

  - id: outline
    type: llm
    dependencies: ["outline_prompt"]
    input_mapping:
      prompt: "{{ nodes.outline_prompt.outputs.output }}"
    parameters:
      system: "You plan factual reports. Answer with JSON only."
      json_schema:
        type: object
        required: [title, sections]
        properties:
          title: { type: string }
          sections:
            type: array
            items:
              type: object
              required: [heading, brief]
              properties:
                heading: { type: string }
                brief: { type: string }

  - id: section_items
    type: template
    dependencies: ["outline", "source"]
    input_mapping:
      plan: "{{ nodes.outline.outputs.output }}"
      source_text: "{{ nodes.source.outputs.output }}"
    parameters:
      output_format: json
      template: |
        {"items": [{% for s in plan.sections %}{"heading": {{ s.heading | json_encode() }}, "brief": {{ s.brief | json_encode() }}, "source": {{ source_text | json_encode() }}}{% if not loop.last %},{% endif %}{% endfor %}]}

  - id: write_sections
    type: map
    dependencies: ["section_items"]
    input_mapping:
      input_list: "{{ nodes.section_items.outputs.items }}"
    parameters:
      parallel: true
      max_concurrent: 4
      template:
        - id: section_prompt
          type: template
          parameters:
            template: |
              Write the report section "{{ item.heading }}" ({{ item.brief }}).
              Two to four paragraphs of Markdown, no heading. Base every
              claim on the source, and list the exact sentences you relied
              on as quotes, each with the id in square brackets of the
              passage it comes from ("" when passages have no ids).

              SOURCE:
              {{ item.source }}
        - id: write
          type: llm
          dependencies: ["section_prompt"]
          input_mapping:
            prompt: "{{ nodes.section_prompt.outputs.output }}"
          parameters:
            system: "You write sourced report sections. Answer with JSON only."
            json_schema:
              type: object
              required: [markdown, quotes]
              properties:
                markdown: { type: string }
                quotes:
                  type: array
                  items:
                    type: object
                    required: [chunk_id, quote]
                    properties:
                      chunk_id: { type: string }
                      quote: { type: string }

  # Sections in outline order, each followed by its footnoted quotes.
  - id: assemble
    type: template
    dependencies: ["write_sections", "outline"]
    input_mapping:
      plan: "{{ nodes.outline.outputs.output }}"
      written: "{{ nodes.write_sections.outputs.results }}"
    parameters:
      template: |
        # {{ plan.title }}
        {% set_global n = 0 %}{% for r in written %}
        ## {{ plan.sections[loop.index0].heading }}

        {{ r.write.Ok.output.value.markdown }}{% for q in r.write.Ok.output.value.quotes %}{% set_global n = n + 1 %}[^{{ n }}]{% endfor %}
        {% endfor %}
        {% set_global n = 0 %}{% for r in written %}{% for q in r.write.Ok.output.value.quotes %}{% set_global n = n + 1 %}
        [^{{ n }}]: "{{ q.quote }}"{% if q.chunk_id %} [{{ q.chunk_id }}]{% endif %}{% if url %} — <{{ url }}>{% endif %}{% endfor %}{% endfor %}

  - id: report_md
    type: render_markdown
    dependencies: ["assemble"]
    input_mapping:
      markdown: "{{ nodes.assemble.outputs.output }}"
    parameters:
      format: md

  - id: report_html
    type: render_markdown
    dependencies: ["assemble"]
    input_mapping:
      markdown: "{{ nodes.assemble.outputs.output }}"

  - id: report_pdf
    type: render_document
    dependencies: ["assemble"]
    run_if: "{{ nodes.options.outputs.want_pdf }}"
    input_mapping:
      markdown: "{{ nodes.assemble.outputs.output }}"
    parameters:
      format: pdf

  # The contract (RFC §4.3): structured content, citations, files.
  - id: deliverable
    type: template
    dependencies: ["report_md", "report_html"]
    input_mapping:
      plan: "{{ nodes.outline.outputs.output }}"
      written: "{{ nodes.write_sections.outputs.results }}"
      md: "{{ nodes.report_md.outputs.report_md_output }}"
      html: "{{ nodes.report_html.outputs.report_html_output }}"
      pdf_file: "{{ nodes.report_pdf.outputs.report_pdf_output }}"
    parameters:
      output_format: json
      template: |
        {"deliverable": {
          "kind": "report",
          "title": {{ plan.title | json_encode() }},
          "content": {"sections": [{% for r in written %}{"heading": {{ plan.sections[loop.index0].heading | json_encode() }}, "markdown": {{ r.write.Ok.output.value.markdown | json_encode() }}}{% if not loop.last %},{% endif %}{% endfor %}]},
          "citations": [{% set_global first = true %}{% for r in written %}{% for q in r.write.Ok.output.value.quotes %}{% if not first %},{% endif %}{% set_global first = false %}{% set_global sid = "" %}{% set_global passage = "" %}{% for c in chunks %}{% if c.chunk_id == q.chunk_id %}{% set_global sid = c.source_id %}{% set_global passage = c.content %}{% endif %}{% endfor %}{"source_id": {% if sid %}{{ sid | json_encode() }}{% elif url %}{{ url | json_encode() }}{% else %}"text"{% endif %}, "chunk_id": {{ q.chunk_id | default(value="") | json_encode() }}, "quote": {{ q.quote | json_encode() }}}{% endfor %}{% endfor %}],
          "files": [
            {"node_id": "report_md", "name": {{ md | split(pat="/") | last | json_encode() }}, "mime_type": "text/markdown", "role": "primary"},
            {"node_id": "report_html", "name": {{ html | split(pat="/") | last | json_encode() }}, "mime_type": "text/html", "role": "preview"}{% if pdf_file %},
            {"node_id": "report_pdf", "name": {{ pdf_file | split(pat="/") | last | json_encode() }}, "mime_type": "application/pdf", "role": "primary"}{% endif %}
          ]
        }}
`,t5=`# Y6.1 — \`slides\` generator (docs/RFC_CONTENT_STUDIO.md §3, §5).
#
# Source (url / text / notebook chunks) → a structured deck (llm,
# json_schema: per slide a title, bullets, speaker notes and an optional
# figure) → each figure rendered by \`figure\` (svg / tikz / typst / d2 / python,
# render errors fed back) → Markdown slides → PPTX through pandoc, with
# the figures embedded as SVG (vector) → a \`Deliverable\`.
#
#   yanshi workflow run examples/generators/slides.yml \\
#     --input url https://example.org/article --output-dir out
#
# Optional \`theme\`: a reference .pptx in the host's YANSHI_THEMES_DIR.
name: slides
description: "A slide deck of a source, with rendered figures, as .pptx"

inputs:
  url:
    description: "Page to present (fetched with SSRF protection); leave empty to use \`text\`"
    default: ""
  text:
    description: "Source text, used when \`url\` is empty"
    default: ""
  chunks:
    description: "Notebook passages to work from instead of url / text: [{chunk_id, source_id, content}]"
    default: []
  focus:
    description: "What the deck should argue or explain"
    default: "the main ideas, for a general audience"
  slides:
    description: "Number of content slides"
    default: 6
  figure_languages:
    description: "Figure languages the deck may use (svg is always available; tikz / typst / d2 / python need their renderers; d2 suits boxes-and-arrows diagrams)"
    default: "svg"
  theme:
    description: "Built-in palette ('dark' or 'light'), empty for default 'dark', or a reference .pptx name in YANSHI_THEMES_DIR"
    default: ""
  enable_illustrations:
    description: "Generate art-directed AI illustrations for slides without an explicit figure"
    default: false
  image_model:
    description: "Text-to-image model used when enable_illustrations is true"
    default: "gemini-3.1-flash-image"
  style_preset:
    description: "Art-direction preset ('editorial_tech' | 'isometric_3d' | 'flat_vector' | 'scientific_diagram' | 'cinematic_keynote')"
    default: "editorial_tech"

nodes:
  - id: options
    type: template
    parameters:
      output_format: json
      template: |
        {
          "has_url": {{ url != "" }},
          "use_native": {{ theme == "" or theme == "dark" or theme == "light" }},
          "use_ref_doc": {{ theme != "" and theme != "dark" and theme != "light" }},
          "palette": {% if theme == "light" %}"light"{% else %}"dark"{% endif %},
          "enable_illustrations": {{ enable_illustrations }},
          "image_model": {{ image_model | json_encode() }}
        }

  # \`url\` (the workflow input) reaches the node directly.
  - id: fetch
    type: http
    dependencies: ["options"]
    run_if: "{{ nodes.options.outputs.has_url }}"

  - id: source
    type: template
    input_mapping:
      page: "{{ nodes.fetch.outputs.body }}"
    parameters:
      template: "{% if chunks | length > 0 %}{% for c in chunks %}[{{ c.chunk_id }}] {{ c.content }}\\n\\n{% endfor %}{% elif page %}{{ page | striptags | truncate(length=24000) }}{% else %}{{ text | truncate(length=24000) }}{% endif %}"

  - id: deck_prompt
    type: template
    dependencies: ["source"]
    input_mapping:
      source_text: "{{ nodes.source.outputs.output }}"
    parameters:
      template: |
        Plan a deck of {{ slides }} content slides on the source below,
        presenting {{ focus }}. Each slide: a short title, 2-5 bullets of
        at most twelve words, speaker notes of two to four sentences, and
        a figure only where a diagram explains better than words. A
        figure gives its language (one of: {{ figure_languages }}) and a
        precise description of what to draw. Use only what the source
        supports.

        SOURCE:
        {{ source_text }}

  - id: deck
    type: llm
    dependencies: ["deck_prompt"]
    input_mapping:
      prompt: "{{ nodes.deck_prompt.outputs.output }}"
    parameters:
      system: "You design clear, sparse slide decks. Answer with JSON only."
      json_schema:
        type: object
        required: [title, slides]
        properties:
          title: { type: string }
          subtitle: { type: string }
          slides:
            type: array
            items:
              type: object
              required: [title, bullets, notes]
              properties:
                title: { type: string }
                bullets: { type: array, items: { type: string } }
                notes: { type: string }
                figure:
                  type: object
                  required: [language, description]
                  properties:
                    language: { type: string }
                    description: { type: string }
                    source: { type: string }

  # One Map item per figure, remembering its slide.
  - id: figure_items
    type: template
    dependencies: ["deck"]
    input_mapping:
      plan: "{{ nodes.deck.outputs.output }}"
    parameters:
      output_format: json
      template: |
        {"items": [{% set_global first = true %}{% for s in plan.slides %}{% if s.figure and s.figure.language %}{% if not first %},{% endif %}{% set_global first = false %}{"slide": {{ loop.index0 }}, "language": {{ s.figure.language | json_encode() }}, "description": {{ s.figure.description | json_encode() }}, "source": {{ s.figure.source | default(value="") | json_encode() }}}{% endif %}{% endfor %}]}

  - id: figures
    type: map
    dependencies: ["figure_items"]
    input_mapping:
      input_list: "{{ nodes.figure_items.outputs.items }}"
    parameters:
      parallel: true
      max_concurrent: 3
      template:
        - id: fig
          type: figure
          input_mapping:
            language: "{{ item.language }}"
            description: "{{ item.description }}"
            source: "{{ item.source }}"
          parameters:
            png: true
            png_width: 1600

  - id: deck_spec
    type: template
    dependencies: ["figures", "deck", "figure_items", "options"]
    input_mapping:
      plan: "{{ nodes.deck.outputs.output }}"
      wanted: "{{ nodes.figure_items.outputs.items }}"
      drawn: "{{ nodes.figures.outputs.results }}"
      palette: "{{ nodes.options.outputs.palette }}"
    parameters:
      output_format: json
      template: |
        {"deck": {
          "title": {{ plan.title | json_encode() }},
          "subtitle": {{ plan.subtitle | default(value="") | json_encode() }},
          "theme": {{ palette | json_encode() }},
          "slides": [
            {% for s in plan.slides %}{% set slide = loop.index0 %}{% set_global fig_src = "" %}{% set_global fig_desc = "" %}{% for w in wanted %}{% if w.slide == slide %}{% set r = drawn | nth(n=loop.index0) %}{% if r.fig and r.fig.Ok %}{% set_global fig_src = r.fig.Ok.fig_svg.path %}{% set_global fig_desc = w.description %}{% endif %}{% endif %}{% endfor %}{
              "title": {{ s.title | json_encode() }},
              "left": {
                "kind": "bullets",
                "items": [{% for b in s.bullets %}{"heading": {{ b | json_encode() }}, "body": ""}{% if not loop.last %},{% endif %}{% endfor %}]
              },
              {% if fig_src != "" %}"right": {"kind": "image", "title": "Figure", "src": {{ fig_src | json_encode() }}, "caption": {{ fig_desc | json_encode() }}},
              "visual_mode": "structured_panel",{% else %}{% set joined_b = s.bullets | join(sep="; ") %}{% set ill_p = s.title ~ ": " ~ joined_b %}"visual_mode": "illustration",
              "illustration_prompt": {{ ill_p | json_encode() }},{% endif %}
              "notes": {{ s.notes | json_encode() }}
            }{% if not loop.last %},{% endif %}{% endfor %}
          ]
        }}

  - id: illustrate
    type: illustration
    dependencies: ["deck_spec", "options"]
    input_mapping:
      deck: "{{ nodes.deck_spec.outputs.deck }}"
      model: "{{ nodes.options.outputs.image_model }}"
      palette: "{{ nodes.options.outputs.palette }}"
      enabled: "{{ nodes.options.outputs.enable_illustrations }}"
    parameters:
      max_images: 3
      max_concurrent: 3
      graceful_fallback: true

  - id: deck_pptx
    type: render_slides
    dependencies: ["illustrate", "options"]
    run_if: "{{ nodes.options.outputs.use_native }}"
    input_mapping:
      deck: "{{ nodes.illustrate.outputs.illustrate_deck }}"
    parameters:
      width: 1920
      progressive: false
      section_cards: false
      export_pptx: true

  # Optional custom Pandoc \`--reference-doc\` path when \`theme\` names a file in YANSHI_THEMES_DIR.
  - id: assemble
    type: template
    dependencies: ["figures", "deck", "figure_items", "options"]
    run_if: "{{ nodes.options.outputs.use_ref_doc }}"
    input_mapping:
      plan: "{{ nodes.deck.outputs.output }}"
      wanted: "{{ nodes.figure_items.outputs.items }}"
      drawn: "{{ nodes.figures.outputs.results }}"
    parameters:
      template: |
        ---
        title: {{ plan.title | json_encode() }}
        {% if plan.subtitle %}subtitle: {{ plan.subtitle | json_encode() }}
        {% endif %}---
        {% for s in plan.slides %}{% set slide = loop.index0 %}
        ## {{ s.title }}

        {% for b in s.bullets %}- {{ b }}
        {% endfor %}{% for w in wanted %}{% if w.slide == slide %}{% set r = drawn[loop.index0] %}{% if r.fig.Ok %}
        ![]({{ r.fig.Ok.fig_svg.path }}){ height=60% }
        {% endif %}{% endif %}{% endfor %}
        ::: notes
        {{ s.notes }}
        :::
        {% endfor %}

  - id: ref_pptx
    type: render_document
    dependencies: ["assemble", "options"]
    run_if: "{{ nodes.options.outputs.use_ref_doc }}"
    input_mapping:
      markdown: "{{ nodes.assemble.outputs.output }}"
    parameters:
      format: pptx

  - id: deliverable
    type: template
    dependencies: ["illustrate"]
    input_mapping:
      plan: "{{ nodes.deck.outputs.output }}"
      drawn: "{{ nodes.figures.outputs.results }}"
      painted: "{{ nodes.illustrate.outputs.illustrate_images }}"
      native_pptx: "{{ nodes.deck_pptx.outputs.deck_pptx_pptx }}"
      themed_pptx: "{{ nodes.ref_pptx.outputs.ref_pptx_output }}"
    parameters:
      output_format: json
      template: |
        {% if native_pptx is defined and native_pptx %}{% set use_native = true %}{% set out_pptx = native_pptx %}{% elif themed_pptx is defined and themed_pptx %}{% set use_native = false %}{% set out_pptx = themed_pptx %}{% else %}{% set use_native = true %}{% set out_pptx = "" %}{% endif %}{"deliverable": {
          "kind": "slides",
          "title": {{ plan.title | json_encode() }},
          "content": {"slides": {{ plan.slides | json_encode() }}},
          "citations": [],
          "files": [
            {"node_id": {% if use_native %}"deck_pptx"{% else %}"ref_pptx"{% endif %}, "name": {{ out_pptx | split(pat="/") | last | json_encode() }}, "mime_type": "application/vnd.openxmlformats-officedocument.presentationml.presentation", "role": "primary"}{% for r in drawn %}{% if r.fig.Ok %},
            {"node_id": "fig", "name": {{ r.fig.Ok.fig_svg.path | split(pat="/") | last | json_encode() }}, "mime_type": "image/svg+xml", "role": "asset"}{% endif %}{% endfor %}{% for img in painted %},
            {"node_id": "illustrate", "name": {{ img.path | split(pat="/") | last | json_encode() }}, "mime_type": {{ img.mime_type | default(value="image/png") | json_encode() }}, "role": "asset"}{% endfor %}
          ]
        }}
`,n5=`# Live-Presenter Keynote Technical Lecture Video Generator (\`tech_slides_video\`).
#
# End-to-end multimodal lecture studio:
#   1. Multi-source grounding (\`web_search\`, optional \`repo_snapshot\` & \`pdf_extract\`)
#   2. Structured Keynote SlideDeck + progressive step-by-step presenter script (\`spoken\` vs \`caption\`)
#   3. Parallel D2 vector architecture diagram (\`figure\`), AI editorial illustrations (\`text_to_image\`),
#      and optional presenter studio portrait / Veo keynote clip (\`spotlight\` | \`pip\` | \`intro_outro\`)
#   4. 1920x1080 vector slide rendering with chapter opener cards, non-overlapping code callouts,
#      spotlight rings, PiP safe zone, and global progress bar (\`render_slides\`)
#   5. Parallel per-step expressive TTS (\`map -> tts\` with vocal \`instructions\`)
#   6. Millisecond-accurate FFmpeg assembly with breathing pauses, smooth slide fades, rounded PiP
#      presenter overlay, SRT + WebVTT subtitles (\`render_video\`), plus \`.pptx\` & \`.pdf\` (\`render_document\`)
#
#   yanshi workflow run examples/generators/tech_slides_video.yml --timeout 15m \\
#     --input topic "Rust Traits 深度讲解：零成本抽象与虚表内存模型" \\
#     --output-dir runs/traits-lecture
name: tech_slides_video
description: "拟真人 PPT 演播级 1080p 技术精讲视频：支持联网/仓库/PDF 多源输入、毫秒级按步声画卡点、视线引导口播、D2 架构图与 AI 插图、聚光灯/画中画/片头出镜三模式及 PPTX/PDF 导出"

inputs:
  topic:
    description: "技术精讲主题"
    default: "Rust Traits 深度讲解：零成本静态单态化与胖指针动态分发"
  audience:
    description: "目标听众群体"
    default: "后端研发工程师与系统架构师"
  narration_style:
    description: "讲师口播风格与现场节奏"
    default: "资深系统架构师现场 Keynote 讲演：逻辑层层递进、善用真实工程判据，自然引导听众视线聚焦屏幕左右栏代码行号与架构节点"
  source_text:
    description: "可选补充笔记或参考文本"
    default: ""
  source_repo:
    description: "可选 GitHub 仓库链接或本地源码目录（自动扫描代码树与模块依赖）"
    default: ""
  source_pdf:
    description: "可选论文/技术文档 PDF 路径或 arXiv 链接（自动分页提取正文）"
    default: ""
  enable_web_search:
    description: "是否联网搜索最新技术细节与工程实践支撑内容"
    default: true
  slide_count:
    description: "正片幻灯片章节页数（4-10）"
    default: 6
  spoken_language:
    description: "课件文字与口播语言"
    default: "中文"
  slide_theme:
    description: "幻灯片配色主题：'dark'（暗色开发者演播风）或 'light'（浅色学术讲义风）"
    default: "dark"
  presenter_mode:
    description: "主讲人呈现模式：'spotlight'（默认全屏聚光灯）| 'pip'（右下角主讲人画中画）| 'intro_outro'（片头片尾主讲人镜头 + 正片全屏幻灯片）"
    default: "spotlight"
  section_cards:
    description: "章节切换时自动插入 Keynote 风格大标题过渡卡与导语配音"
    default: true
  enable_d2_diagram:
    description: "为核心机制页自动编译高精度 D2 矢量架构图"
    default: true
  enable_illustrations:
    description: "为开篇引入或总结页调用文生图生成无文字概念插图"
    default: true
  chat_model:
    description: "主策划与脚本生成模型（推荐 gemini-2.5-flash / gemini-2.5-pro，兼容 qwen-plus / deepseek-v4-flash / glm-5.1 / kimi-k2.6）"
    default: "gemini-2.5-flash"
  tts_model:
    description: "语音合成模型（推荐 gemini-2.5-flash-preview-tts，兼容 qwen3-tts-flash / step-tts-mini / gpt-4o-mini-tts）"
    default: "gemini-2.5-flash-preview-tts"
  voice:
    description: "主讲人音色（如 Kore / Charon / Puck，跨厂商切换时自动归一化兜底）"
    default: "Kore"
  tts_instructions:
    description: "TTS 演播语气导演指令（Gemini TTS 与 OpenAI TTS 原生支持，其他厂商自动忽略）"
    default: "Speak as a seasoned, engaging systems architect delivering a live technical keynote at a brisk, natural conversational pace, with clear emphasis on technical terms and natural breathing cadence."
  image_model:
    description: "文生图模型（用于概念插图与主讲人形象，推荐 gemini-3.1-flash-image，兼容 wan2.5-t2i-preview / glm-image）"
    default: "gemini-3.1-flash-image"
  image_size:
    description: "生成插图画面比例或尺寸"
    default: "16:9"
  style_preset:
    description: "插图艺术指导预设：'editorial_tech' | 'isometric_3d' | 'flat_vector' | 'blueprint_schematic' | 'scientific_diagram' | 'cinematic_keynote'"
    default: "editorial_tech"
  visual_style:
    description: "AI 插图补充视觉艺术风格（与 style_preset 叠加生效）"
    default: "glowing copper-orange and cyan conduits, crisp architectural geometry, studio rim lighting"
  refine_prompt:
    description: "是否先经艺术总监模型精炼视觉隐喻提示词"
    default: true
  video_model:
    description: "可选动态主讲人视频模型（留空默认使用高清演播室讲师形象卡，兼容所有厂商 Key；如有视频权限可填 veo-3.1-fast-generate-001 / kling-4.0-flash / hailuo-02）"
    default: ""
  presenter_prompt:
    description: "主讲人演播室镜头画面提示词（仅在 pip / intro_outro 模式下使用）"
    default: "A confident software architect speaking naturally on a modern dark developer keynote stage, subtle warm rim light and soft cyan background bokeh, waist-up framing, calm hand gesture, photorealistic 16:9, strictly no text or logos"
  step_pause_ms:
    description: "页内动画步之间的自然呼吸停顿时长（毫秒）"
    default: 260
  slide_pause_ms:
    description: "翻页边界的自然留白时长（毫秒）"
    default: 550
  burn_subtitles:
    description: "是否将半透明演播底衬字幕直接烧录进视频帧（默认 false 封装可开关软字幕轨 + .srt + .vtt）"
    default: false
  export_pptx:
    description: "导出可编辑 PowerPoint (.pptx) 课件（含高清帧与讲师逐字稿备注）"
    default: true
  export_pdf:
    description: "导出高清排版讲义 (.pdf)"
    default: true

nodes:
  - id: mode_flags
    type: template
    parameters:
      output_format: json
      template: |
        {
          "do_search": {{ enable_web_search }},
          "has_repo": {{ source_repo != "" }},
          "has_pdf": {{ source_pdf != "" }},
          "do_d2": {{ enable_d2_diagram }},
          "is_pip": {{ presenter_mode == "pip" }},
          "is_intro_outro": {{ presenter_mode == "intro_outro" }},
          "need_presenter": {{ presenter_mode == "pip" or presenter_mode == "intro_outro" }},
          "need_presenter_video": {{ (presenter_mode == "pip" or presenter_mode == "intro_outro") and video_model != "" }},
          "source_repo": {{ source_repo | json_encode() }},
          "source_pdf": {{ source_pdf | json_encode() }},
          "chat_model": {{ chat_model | json_encode() }},
          "slide_theme": {{ slide_theme | json_encode() }},
          "d2_theme": {% if slide_theme == "dark" %}"200"{% else %}"0"{% endif %},
          "enable_illustrations": {{ enable_illustrations }},
          "visual_style": {{ visual_style | json_encode() }},
          "image_model": {{ image_model | json_encode() }},
          "image_size": {{ image_size | json_encode() }},
          "video_model": {{ video_model | json_encode() }},
          "presenter_prompt": {{ presenter_prompt | json_encode() }},
          "burn_subtitles": {{ burn_subtitles }},
          "pptx": {{ export_pptx }},
          "pdf": {{ export_pdf }}
        }

  - id: search_query
    type: template
    parameters:
      template: "{{ topic }} 底层原理 核心机制 内存模型 工程最佳实践"

  - id: research
    type: web_search
    dependencies: ["mode_flags", "search_query"]
    run_if: "{{ nodes.mode_flags.outputs.do_search }}"
    input_mapping:
      query: "{{ nodes.search_query.outputs.output }}"
    parameters:
      max_results: 6

  - id: repo_scan
    type: repo_snapshot
    dependencies: ["mode_flags"]
    run_if: "{{ nodes.mode_flags.outputs.has_repo }}"
    input_mapping:
      source: "{{ nodes.mode_flags.outputs.source_repo }}"
    parameters:
      max_files: 60

  - id: pdf_scan
    type: pdf_extract
    dependencies: ["mode_flags"]
    run_if: "{{ nodes.mode_flags.outputs.has_pdf }}"
    input_mapping:
      source: "{{ nodes.mode_flags.outputs.source_pdf }}"
    parameters:
      max_pages: 25

  - id: grounded_context
    type: template
    dependencies: ["mode_flags"]
    input_mapping:
      web_md: "{{ nodes.research.outputs.markdown }}"
      repo_md: "{{ nodes.repo_scan.outputs.markdown }}"
      pdf_md: "{{ nodes.pdf_scan.outputs.markdown }}"
    parameters:
      template: |
        {% if source_text != "" %}
        【用户补充参考资料】
        {{ source_text | truncate(length=10000) }}
        {% endif %}
        {% if repo_md is defined and repo_md %}
        【源码仓库结构与依赖快照】
        {{ repo_md | truncate(length=12000) }}
        {% endif %}
        {% if pdf_md is defined and pdf_md %}
        【PDF 论文/技术文档摘录】
        {{ pdf_md | truncate(length=12000) }}
        {% endif %}
        {% if web_md is defined and web_md %}
        【联网检索技术资料】
        {{ web_md | truncate(length=10000) }}
        {% endif %}

  - id: presenter_portrait
    type: text_to_image
    dependencies: ["mode_flags"]
    run_if: "{{ nodes.mode_flags.outputs.need_presenter }}"
    input_mapping:
      visual: "{{ nodes.mode_flags.outputs.presenter_prompt }}"
      model: "{{ nodes.mode_flags.outputs.image_model }}"
      size: "{{ nodes.mode_flags.outputs.image_size }}"
    parameters:
      prompt: "{{visual}}"

  - id: presenter_clip
    type: text_to_video
    dependencies: ["mode_flags", "presenter_portrait"]
    run_if: "{{ nodes.mode_flags.outputs.need_presenter_video }}"
    input_mapping:
      model: "{{ nodes.mode_flags.outputs.video_model }}"
      image: "{{ nodes.presenter_portrait.outputs.presenter_portrait_image }}"
    parameters:
      prompt: "Subtle natural head nod, calm speaking lip movement, gentle keynote presenter hand gesture, steady studio camera, soft bokeh"
      aspect_ratio: "16:9"
      duration_seconds: 6

  - id: plan_prompt
    type: template
    dependencies: ["grounded_context"]
    input_mapping:
      ref_pack: "{{ nodes.grounded_context.outputs.output }}"
    parameters:
      template: |
        你是一位享誉业界的资深系统架构师与技术 Keynote 主讲人。请围绕主题「{{ topic }}」，面向「{{ audience }}」，以「{{ narration_style }}」风格，设计一套共 {{ slide_count }} 页幻灯片的演播级技术精讲课件（语言：{{ spoken_language }}，配色主题：{{ slide_theme }}）。

        核心设计准则（极其重要，直接决定声画卡点与视觉表现）：
        1. **工程级双栏可视化面板（拒绝空洞口号）**：
           - 每页必须提供 \`left\` 主面板与 \`right\` 备选结构化面板（当未启用外部插图或处于结构化对比页时直接并排渲染）：
             * \`code\`：真实可编译的核心代码（控制在 8-15 行，2 空格缩进），配合 1-3 个精炼 \`callouts\`（\`{"line": 行号, "text": "≤8字批注", "tone": "rust|cyan|green"}\`）；
             * \`flowchart\`：编译流水线、调用链或决策拓扑（4-6 个 \`nodes\` 含 \`id\`、\`label\` 与简短 \`sublabel\`，配 \`edges\` 有向边说明）；
             * \`memory\`：栈/堆/虚表/结构体真实字节偏移布局（严格控制在 2-3 个 \`regions\`，含 \`name\` 与 \`slots: [{"id": "s1", "offset": "+0x00", "label": "data_ptr", "value": "0x7ffd..."}]\`，配 \`pointers: [{"from": "s1", "to": "h1", "label": "points to"}]\`）；
             * \`table\`：多方案横向对比矩阵（单元格以 \`✓\` / \`✗\` / \`⚠\` 开头标识优劣，设 \`highlight_col\` 突出主角列）；
             * \`bullets\`：带 \`badge\` 标签的高密度架构卡片（可设 \`grid: true\` 排成 2×2 全景知识象限用于开篇或收官页）；
           - **严禁输出空面板**：即使本页 \`visual_mode\` 为 \`"illustration"\` 或 \`"arch_diagram"\`，\`left\` 主面板也必须完整填充对应类型数据（\`bullets\` 的 \`items\` 至少 3-4 条完整卡片、\`code\` 非空、\`flowchart\` 的 \`nodes\` 至少 4 个、\`memory\` 的 \`regions\` 至少 2 个、\`table\` 的 \`rows\` 至少 3 行），绝不允许出现空数组或空白左栏。
        2. **混合视觉编排（\`visual_mode\`）**：
           - 全套课件挑选恰好 1 页最核心的系统架构或数据流页设为 \`visual_mode: "arch_diagram"\`，并在顶层 \`arch_diagram_spec\` 中详细描述该页 D2 架构图的分层容器、核心组件与调用边（8-12 个节点）；
           - 挑选 1-2 页（如第 1 页破题引入或末页总结升华）设为 \`visual_mode: "illustration"\`，提供纯英文视觉隐喻提示词 \`illustration_prompt\`（严禁包含任何文字、字母或水印）与简短中文图注 \`illustration_caption\`；
           - 其余硬核机制拆解页全部设为 \`visual_mode: "structured_panel"\`（左右双栏硬核对照，如左 \`code\` + 右 \`memory\` / \`flowchart\`）。
        3. **拟真人逐动画步声画卡点脚本（\`steps\` 数组，每页严格 2-4 步）**：
           - 每一步对应视频里的一个渐进点亮画面与一段独立合成的语音，必须同时给出：
             * \`caption\`：屏幕底栏/字幕展示的凝练短句（≤32 字，可保留精准代码符号如 \`&dyn Trait\`）；
             * \`spoken\`：**真人讲师现场口播稿**（1-2 句自然流畅的口语化讲解，必须包含自然的视线引导词——例如“请大家看屏幕左侧第 2 到 4 行”“注意看右边虚表里的这个函数指针”“对比表格第二列我们可以发现”——并且把生硬符号转化为自然口语读法，如把 \`->\` 写成“返回”、把 \`&dyn\` 写成“dyn 动态引用”，严禁出现括号舞台提示词）；
             * \`code_lines\`：若本页含 \`code\` 面板，填写当前步聚光灯高亮的 \`[起始行号, 结束行号]\`（1-based）；
             * \`focus_ids\`：若本页含 \`flowchart\` 或 \`memory\` 面板，填写当前步重点点亮的节点/槽位 \`id\` 列表；
             * \`left_reveal\` / \`right_reveal\`：当前步揭晓的条目/节点数量；最后一步设 \`show_takeaway: true\`。
        4. 每页提供完整的 \`notes\`（汇总整页讲稿供 PowerPoint 备注页使用）与一句一针见血的 \`takeaway\` 工程判据。
        {% if ref_pack | trim != "" %}
        5. 充分吸收并忠实引用以下真实技术背景资料（严禁编造虚假 API）：
        {{ ref_pack | truncate(length=22000) }}
        {% endif %}

  - id: deck
    type: llm
    dependencies: ["plan_prompt", "mode_flags"]
    input_mapping:
      prompt: "{{ nodes.plan_prompt.outputs.output }}"
      model: "{{ nodes.mode_flags.outputs.chat_model }}"
    parameters:
      max_tokens: 16384
      system: "你是资深系统架构师与 Keynote 演播编导，仅输出符合 SlideDeck JSON Schema 的严格合法 JSON。"
      json_schema:
        type: object
        required: [title, slides]
        properties:
          title: { type: string }
          subtitle: { type: string }
          author: { type: string }
          theme: { type: string }
          arch_diagram_spec:
            type: string
            description: "核心机制页的 D2 矢量架构图绘制说明（指明分层容器、组件名与有向调用边）"
          slides:
            type: array
            minItems: 1
            maxItems: 12
            items:
              type: object
              required: [title, left, right, visual_mode, takeaway, notes, steps]
              properties:
                section: { type: string }
                title: { type: string }
                subtitle: { type: string }
                left:
                  type: object
                  required: [kind]
                  properties:
                    kind:
                      type: string
                      enum: [bullets, code, flowchart, memory, table]
                    title: { type: string }
                    grid: { type: boolean }
                    items:
                      type: array
                      items:
                        type: object
                        required: [heading, body]
                        properties:
                          badge: { type: string }
                          heading: { type: string }
                          body: { type: string }
                          tone: { type: string }
                    language: { type: string }
                    code: { type: string }
                    callouts:
                      type: array
                      items:
                        type: object
                        required: [line, text]
                        properties:
                          line: { type: integer }
                          text: { type: string }
                          tone: { type: string }
                    direction: { type: string }
                    nodes:
                      type: array
                      items:
                        type: object
                        required: [id, label]
                        properties:
                          id: { type: string }
                          label: { type: string }
                          sublabel: { type: string }
                          tone: { type: string }
                    edges:
                      type: array
                      items:
                        type: object
                        required: [from, to]
                        properties:
                          from: { type: string }
                          to: { type: string }
                          label: { type: string }
                          dashed: { type: boolean }
                    regions:
                      type: array
                      items:
                        type: object
                        required: [name, slots]
                        properties:
                          name: { type: string }
                          subtitle: { type: string }
                          tone: { type: string }
                          slots:
                            type: array
                            items:
                              type: object
                              required: [id, label, value]
                              properties:
                                id: { type: string }
                                offset: { type: string }
                                label: { type: string }
                                value: { type: string }
                    pointers:
                      type: array
                      items:
                        type: object
                        required: [from, to]
                        properties:
                          from: { type: string }
                          to: { type: string }
                          label: { type: string }
                          tone: { type: string }
                    highlight_col: { type: integer }
                    columns:
                      type: array
                      items: { type: string }
                    rows:
                      type: array
                      items:
                        type: array
                        items: { type: string }
                right:
                  type: object
                  required: [kind]
                  properties:
                    kind:
                      type: string
                      enum: [bullets, code, flowchart, memory, table]
                    title: { type: string }
                    grid: { type: boolean }
                    items:
                      type: array
                      items:
                        type: object
                        required: [heading, body]
                        properties:
                          badge: { type: string }
                          heading: { type: string }
                          body: { type: string }
                          tone: { type: string }
                    language: { type: string }
                    code: { type: string }
                    callouts:
                      type: array
                      items:
                        type: object
                        required: [line, text]
                        properties:
                          line: { type: integer }
                          text: { type: string }
                          tone: { type: string }
                    direction: { type: string }
                    nodes:
                      type: array
                      items:
                        type: object
                        required: [id, label]
                        properties:
                          id: { type: string }
                          label: { type: string }
                          sublabel: { type: string }
                          tone: { type: string }
                    edges:
                      type: array
                      items:
                        type: object
                        required: [from, to]
                        properties:
                          from: { type: string }
                          to: { type: string }
                          label: { type: string }
                          dashed: { type: boolean }
                    regions:
                      type: array
                      items:
                        type: object
                        required: [name, slots]
                        properties:
                          name: { type: string }
                          subtitle: { type: string }
                          tone: { type: string }
                          slots:
                            type: array
                            items:
                              type: object
                              required: [id, label, value]
                              properties:
                                id: { type: string }
                                offset: { type: string }
                                label: { type: string }
                                value: { type: string }
                    pointers:
                      type: array
                      items:
                        type: object
                        required: [from, to]
                        properties:
                          from: { type: string }
                          to: { type: string }
                          label: { type: string }
                          tone: { type: string }
                    highlight_col: { type: integer }
                    columns:
                      type: array
                      items: { type: string }
                    rows:
                      type: array
                      items:
                        type: array
                        items: { type: string }
                split_ratio: { type: number }
                visual_mode:
                  type: string
                  enum: [structured_panel, arch_diagram, illustration]
                illustration_prompt: { type: string }
                illustration_caption: { type: string }
                takeaway: { type: string }
                notes: { type: string }
                steps:
                  type: array
                  minItems: 1
                  maxItems: 5
                  items:
                    type: object
                    required: [caption, spoken]
                    properties:
                      caption: { type: string }
                      spoken: { type: string }
                      left_reveal: { type: integer }
                      right_reveal: { type: integer }
                      code_lines:
                        type: array
                        items: { type: integer }
                      focus_ids:
                        type: array
                        items: { type: string }
                      show_takeaway: { type: boolean }

  - id: arch_figure_desc
    type: template
    dependencies: ["deck"]
    input_mapping:
      plan: "{{ nodes.deck.outputs.output }}"
    parameters:
      template: |
        为主题「{{ plan.title }}」绘制一张结构紧凑清晰的 D2 系统架构与核心机制图（\`direction: right\`，严格控制在 5-8 个节点以内；若需分组容器请直接写 \`group_id: "分组标题" { node_a: "组件名" }\`，严禁使用 \`container\` 关键字，跨分组连线必须写在所有 \`{}\` 之外的顶层并带完整路径 \`group_a.node_x -> group_b.node_y\`；保持默认主题配色）：
        {{ plan.arch_diagram_spec | default(value=plan.title) }}

  - id: arch_figure
    type: figure
    dependencies: ["mode_flags", "arch_figure_desc"]
    run_if: "{{ nodes.mode_flags.outputs.do_d2 }}"
    input_mapping:
      description: "{{ nodes.arch_figure_desc.outputs.output }}"
      model: "{{ nodes.mode_flags.outputs.chat_model }}"
      d2_theme: "{{ nodes.mode_flags.outputs.d2_theme }}"
    parameters:
      language: d2
      png: true
      png_width: 1600
      max_attempts: 4

  - id: illustrate
    type: illustration
    dependencies: ["deck", "mode_flags"]
    input_mapping:
      deck: "{{ nodes.deck.outputs.output }}"
      arch_image: "{{ nodes.arch_figure.outputs.arch_figure_svg }}"
      model: "{{ nodes.mode_flags.outputs.image_model }}"
      prompt_model: "{{ nodes.mode_flags.outputs.chat_model }}"
      size: "{{ nodes.mode_flags.outputs.image_size }}"
      style: "{{ nodes.mode_flags.outputs.visual_style }}"
      theme: "{{ nodes.mode_flags.outputs.slide_theme }}"
      enabled: "{{ nodes.mode_flags.outputs.enable_illustrations }}"
    parameters:
      arch_title: "核心系统架构图 (D2)"
      max_images: 3
      max_concurrent: 3
      graceful_fallback: true

  - id: slides
    type: render_slides
    dependencies: ["illustrate", "mode_flags"]
    input_mapping:
      deck: "{{ nodes.illustrate.outputs.illustrate_deck }}"
      pip_safe_zone: "{{ nodes.mode_flags.outputs.is_pip }}"
      takeaway_only_bar: "{{ nodes.mode_flags.outputs.burn_subtitles }}"
    parameters:
      width: 1920
      progressive: true

  - id: tts_items
    type: template
    dependencies: ["slides", "illustrate", "mode_flags"]
    input_mapping:
      beats: "{{ nodes.slides.outputs.slides_steps }}"
      plan: "{{ nodes.illustrate.outputs.illustrate_deck }}"
      is_intro_outro: "{{ nodes.mode_flags.outputs.is_intro_outro }}"
      p_img: "{{ nodes.presenter_portrait.outputs.presenter_portrait_image }}"
      p_vid: "{{ nodes.presenter_clip.outputs.presenter_clip_video }}"
    parameters:
      output_format: json
      template: |
        {% set host_vid = p_vid | default(value="") %}{% set host_img = p_img | default(value="") %}{% set has_host = is_intro_outro and (host_vid != "" or host_img != "") %}{% set intro_spoken = "大家好，欢迎来到《" ~ plan.title ~ "》技术精讲。今天我们将从核心设计动机出发，结合底层代码与架构模型，一步步拆解这套关键工程机制。" %}{% set intro_cap = plan.title ~ " · 主讲开场导言" %}{% set outro_spoken = "以上就是《" ~ plan.title ~ "》的核心脉络与工程判据复盘。希望这套底层拆解能直接助力你的日常架构实践，感谢观看，我们下期再见。" %}{"items": [
          {% if has_host %}{
            "narration": {{ intro_spoken | json_encode() }},
            "caption": {{ intro_cap | json_encode() }},
            "image": {{ host_img | json_encode() }},
            "video": {{ host_vid | json_encode() }},
            "is_slide_start": true,
            "is_slide_end": true,
            "hide_pip": true,
            "tts_model": {{ tts_model | json_encode() }},
            "voice": {{ voice | json_encode() }},
            "instructions": {{ tts_instructions | json_encode() }}
          },{% endif %}
          {% for b in beats %}{% set beat_spoken = b.spoken | default(value=b.caption) %}{
            "narration": {{ beat_spoken | json_encode() }},
            "caption": {{ b.caption | json_encode() }},
            "image": {{ b.image | json_encode() }},
            "video": "",
            "is_slide_start": {{ b.is_slide_start | json_encode() }},
            "is_slide_end": {{ b.is_slide_end | json_encode() }},
            "hide_pip": false,
            "tts_model": {{ tts_model | json_encode() }},
            "voice": {{ voice | json_encode() }},
            "instructions": {{ tts_instructions | json_encode() }}
          }{% if not loop.last or has_host %},{% endif %}{% endfor %}
          {% if has_host %}{
            "narration": {{ outro_spoken | json_encode() }},
            "caption": "核心脉络复盘与工程结语",
            "image": {{ host_img | json_encode() }},
            "video": {{ host_vid | json_encode() }},
            "is_slide_start": true,
            "is_slide_end": true,
            "hide_pip": true,
            "tts_model": {{ tts_model | json_encode() }},
            "voice": {{ voice | json_encode() }},
            "instructions": {{ tts_instructions | json_encode() }}
          }{% endif %}
        ]}

  - id: narrate
    type: map
    dependencies: ["tts_items"]
    input_mapping:
      input_list: "{{ nodes.tts_items.outputs.items }}"
    parameters:
      parallel: true
      max_concurrent: 2
      template:
        - id: say
          type: tts
          input_mapping:
            input_template: "{{ item.narration }}"
            model: "{{ item.tts_model }}"
            voice: "{{ item.voice }}"
            instructions: "{{ item.instructions }}"
          parameters:
            response_format: wav

  - id: cut
    type: template
    dependencies: ["tts_items", "narrate", "mode_flags"]
    input_mapping:
      beats: "{{ nodes.tts_items.outputs.items }}"
      voiced: "{{ nodes.narrate.outputs.results }}"
      is_pip: "{{ nodes.mode_flags.outputs.is_pip }}"
      p_img: "{{ nodes.presenter_portrait.outputs.presenter_portrait_image }}"
      p_vid: "{{ nodes.presenter_clip.outputs.presenter_clip_video }}"
    parameters:
      output_format: json
      template: |
        {% set pip_v = p_vid | default(value="") %}{% set pip_i = p_img | default(value="") %}{
          "pip_video": {% if is_pip and pip_v != "" %}{{ pip_v | json_encode() }}{% else %}""{% endif %},
          "pip_image": {% if is_pip and pip_i != "" %}{{ pip_i | json_encode() }}{% else %}""{% endif %},
          "scenes": [
            {% for b in beats %}{% set r = voiced | nth(n=loop.index0) %}{
              "audio": {{ r.say.Ok.say_output.path | json_encode() }},
              "caption": {{ b.caption | json_encode() }},
              {% if b.video != "" %}"video": {{ b.video | json_encode() }},{% else %}"image": {{ b.image | json_encode() }},{% endif %}
              "is_slide_start": {{ b.is_slide_start | json_encode() }},
              "is_slide_end": {{ b.is_slide_end | json_encode() }},
              "hide_pip": {{ b.hide_pip | json_encode() }}
            }{% if not loop.last %},{% endif %}{% endfor %}
          ]
        }

  - id: film
    type: render_video
    dependencies: ["cut"]
    input_mapping:
      scenes: "{{ nodes.cut.outputs.scenes }}"
      pip_video: "{{ nodes.cut.outputs.pip_video }}"
      pip_image: "{{ nodes.cut.outputs.pip_image }}"
    parameters:
      width: 1920
      height: 1080
      transition: fade

  - id: deliverable
    type: template
    dependencies: ["illustrate", "film", "slides"]
    input_mapping:
      plan: "{{ nodes.illustrate.outputs.illustrate_deck }}"
      painted: "{{ nodes.illustrate.outputs.illustrate_images }}"
      mp4: "{{ nodes.film.outputs.film_output }}"
      srt: "{{ nodes.film.outputs.film_subtitles }}"
      vtt: "{{ nodes.film.outputs.film_vtt }}"
      seconds: "{{ nodes.film.outputs.film_seconds }}"
      html: "{{ nodes.slides.outputs.slides_html }}"
      arch_svg: "{{ nodes.arch_figure.outputs.arch_figure_svg }}"
      pptx: "{{ nodes.slides.outputs.slides_pptx }}"
      pdf: "{{ nodes.slides.outputs.slides_pdf }}"
    parameters:
      output_format: json
      template: |
        {"deliverable": {
          "kind": "video_overview",
          "title": {{ plan.title | json_encode() }},
          "content": {
            "slides": {{ plan.slides | json_encode() }},
            "seconds": {{ seconds | json_encode() }},
            "presenter_mode": {{ presenter_mode | json_encode() }}
          },
          "citations": [],
          "files": [
            {"node_id": "film", "name": {{ mp4 | split(pat="/") | last | json_encode() }}, "mime_type": "video/mp4", "role": "primary"},
            {"node_id": "film", "name": {{ srt | split(pat="/") | last | json_encode() }}, "mime_type": "application/x-subrip", "role": "asset"}{% if vtt is defined and vtt %},
            {"node_id": "film", "name": {{ vtt | split(pat="/") | last | json_encode() }}, "mime_type": "text/vtt", "role": "asset"}{% endif %},
            {"node_id": "slides", "name": {{ html | split(pat="/") | last | json_encode() }}, "mime_type": "text/html", "role": "preview"}{% if arch_svg is defined and arch_svg %},
            {"node_id": "arch_figure", "name": {{ arch_svg | split(pat="/") | last | json_encode() }}, "mime_type": "image/svg+xml", "role": "asset"}{% endif %}{% for img in painted %},
            {"node_id": "illustrate", "name": {{ img.path | split(pat="/") | last | json_encode() }}, "mime_type": {{ img.mime_type | default(value="image/png") | json_encode() }}, "role": "asset"}{% endfor %}{% if pptx is defined and pptx %},
            {"node_id": "slides", "name": {{ pptx | split(pat="/") | last | json_encode() }}, "mime_type": "application/vnd.openxmlformats-officedocument.presentationml.presentation", "role": "primary"}{% endif %}{% if pdf is defined and pdf %},
            {"node_id": "slides", "name": {{ pdf | split(pat="/") | last | json_encode() }}, "mime_type": "application/pdf", "role": "primary"}{% endif %}
          ]
        }}
`,s5=`# Y6.4 — \`video_overview\` generator (docs/RFC_CONTENT_STUDIO.md §3).
#
# Source (a URL, fetched with the http node's SSRF filter, pasted text,
# or notebook chunks) → a scene-by-scene script (llm, json_schema: what
# is said, what is shown) → per scene, in parallel, a picture
# (text_to_image) and its narration (tts) → render_video (ffmpeg,
# sandboxed): each picture held for its narration, joined into one MP4
# with the narration as a soft subtitle track, plus an .srt.
#
# With \`video_model\` set, the script may mark one or two scenes as moving
# shots; those get a short text_to_video clip (looped for the narration)
# instead of a picture. Clips are billed per second, so this is off by
# default.
#
#   yanshi workflow run examples/generators/video_overview.yml \\
#     --input url https://example.org/article \\
#     --input image_model <text-to-image model> \\
#     --input tts_model <tts model> --input voice <voice> --output-dir out
name: video_overview
description: "A narrated video overview of a source: illustrated scenes, voice-over and subtitles, as MP4"

inputs:
  url:
    description: "Page to present (fetched with SSRF protection); leave empty to use \`text\`"
    default: ""
  text:
    description: "Source text, used when \`url\` is empty"
    default: ""
  focus:
    description: "What the video should explain"
    default: "the main ideas, in plain language"
  scene_count:
    description: "Number of scenes (4-10); each is about 10-20 seconds"
    default: 6
  language:
    description: "Language of the narration"
    default: "English"
  style:
    description: "Visual style every picture shares"
    default: "clean flat editorial illustration, soft colours, no text or letters"
  image_model:
    description: "Registry name of the text-to-image model"
    default: "gpt-image-2"
  image_size:
    description: "Picture size the image model is asked for (pictures are letterboxed to the video)"
    default: "1536x1024"
  tts_model:
    description: "Registry name of the text-to-speech model"
    default: "tts-1"
  voice:
    description: "Narrator voice"
    default: "alloy"
  video_model:
    description: "Registry name of a text-to-video model; empty (the default) keeps every scene a still picture"
    default: ""
  video_seconds:
    description: "Length of each generated clip, in seconds (it loops for the narration)"
    default: 5
  burn_subtitles:
    description: "Draw the narration into the picture instead of a subtitle track the viewer toggles (needs ffmpeg with libass)"
    default: false
  chunks:
    description: "Notebook passages to work from instead of url / text: [{chunk_id, source_id, content}]. Citations then point at these ids."
    default: []

nodes:
  - id: options
    type: template
    parameters:
      output_format: json
      template: '{"has_url": {{ url != "" }}}'

  # \`url\` (the workflow input) reaches the node directly.
  - id: fetch
    type: http
    dependencies: ["options"]
    run_if: "{{ nodes.options.outputs.has_url }}"

  - id: source
    type: template
    input_mapping:
      page: "{{ nodes.fetch.outputs.body }}"
    parameters:
      template: "{% if chunks | length > 0 %}{% for c in chunks %}[{{ c.chunk_id }}] {{ c.content }}\\n\\n{% endfor %}{% elif page %}{{ page | striptags | truncate(length=24000) }}{% else %}{{ text | truncate(length=24000) }}{% endif %}"

  - id: script_prompt
    type: template
    dependencies: ["source"]
    input_mapping:
      source_text: "{{ nodes.source.outputs.output }}"
    parameters:
      template: |
        Write a short explainer video in {{ language }} about the source
        below, covering {{ focus }}, as {{ scene_count }} scenes. For each
        scene: \`narration\`, what the narrator says (two or three spoken
        sentences, no stage directions); and \`visual\`, one sentence
        describing a single still picture that illustrates it (a scene,
        object or diagram-like composition; never text, letters or
        logos). The first scene introduces the subject, the last sums up.
        {% if video_model %}Set \`"motion": true\` on the one or two scenes
        whose picture most needs movement to be understood (a process, a
        change over time); leave it out everywhere else.
        {% endif %}        Every fact must come from the source; when its passages carry
        [ids], give each scene the chunk_id of its passage.

        SOURCE:
        {{ source_text }}

  - id: script
    type: llm
    dependencies: ["script_prompt"]
    input_mapping:
      prompt: "{{ nodes.script_prompt.outputs.output }}"
    parameters:
      system: "You write scripts for short explainer videos. Answer with JSON only."
      json_schema:
        type: object
        required: [title, scenes]
        properties:
          title: { type: string }
          scenes:
            type: array
            minItems: 1
            maxItems: 12
            items:
              type: object
              required: [narration, visual]
              properties:
                narration: { type: string }
                visual: { type: string }
                motion: { type: boolean }
                chunk_id: { type: string }

  # One Map item per scene, carrying what its picture and voice need (a
  # Map body sees only \`item\`, not the workflow inputs).
  - id: shots
    type: template
    dependencies: ["script"]
    input_mapping:
      plan: "{{ nodes.script.outputs.output }}"
    parameters:
      output_format: json
      template: |
        {"items": [{% for s in plan.scenes %}{% set visual = s.visual ~ " Style: " ~ style ~ "." %}{"visual": {{ visual | json_encode() }}, "narration": {{ s.narration | json_encode() }}, "image_model": {{ image_model | json_encode() }}, "image_size": {{ image_size | json_encode() }}, "tts_model": {{ tts_model | json_encode() }}, "voice": {{ voice | json_encode() }}, "moving": {% if s.motion and video_model %}true{% else %}false{% endif %}, "video_model": {{ video_model | json_encode() }}, "video_seconds": {{ video_seconds | json_encode() }}}{% if not loop.last %},{% endif %}{% endfor %}]}

  - id: produce
    type: map
    dependencies: ["shots"]
    input_mapping:
      input_list: "{{ nodes.shots.outputs.items }}"
    parameters:
      parallel: true
      max_concurrent: 4
      template:
        # \`run_if\` reads node outputs, not \`item\`, so the choice is
        # surfaced as an output first.
        - id: mode
          type: template
          input_mapping:
            moving: "{{ item.moving }}"
          parameters:
            output_format: json
            template: '{"moving": {{ moving }}}'
        - id: picture
          type: text_to_image
          dependencies: ["mode"]
          run_if: "{{ !nodes.mode.outputs.moving }}"
          input_mapping:
            visual: "{{ item.visual }}"
            model: "{{ item.image_model }}"
            size: "{{ item.image_size }}"
          parameters:
            prompt: "{{visual}}"
        - id: clip
          type: text_to_video
          dependencies: ["mode"]
          run_if: "{{ nodes.mode.outputs.moving }}"
          input_mapping:
            visual: "{{ item.visual }}"
            model: "{{ item.video_model }}"
            duration_seconds: "{{ item.video_seconds }}"
          parameters:
            prompt: "{{visual}}"
        - id: say
          type: tts
          input_mapping:
            input_template: "{{ item.narration }}"
            voice: "{{ item.voice }}"
            model: "{{ item.tts_model }}"

  # The scenes, in script order, as render_video takes them.
  - id: cut
    type: template
    dependencies: ["produce"]
    input_mapping:
      plan: "{{ nodes.script.outputs.output }}"
      made: "{{ nodes.produce.outputs.results }}"
    parameters:
      output_format: json
      template: |
        {"scenes": [{% for r in made %}{% set s = plan.scenes | nth(n=loop.index0) %}{"audio": {{ r.say.Ok.say_output.path | json_encode() }}, "caption": {{ s.narration | json_encode() }}, {% if r.clip.Ok %}"video": {{ r.clip.Ok.clip_video.path | json_encode() }}{% else %}"image": {{ r.picture.Ok.picture_image.path | json_encode() }}{% endif %}}{% if not loop.last %},{% endif %}{% endfor %}]}

  # \`burn_subtitles\` (the workflow input) reaches the node directly.
  - id: film
    type: render_video
    dependencies: ["cut"]
    input_mapping:
      scenes: "{{ nodes.cut.outputs.scenes }}"

  - id: deliverable
    type: template
    dependencies: ["film"]
    input_mapping:
      plan: "{{ nodes.script.outputs.output }}"
      mp4: "{{ nodes.film.outputs.film_output }}"
      srt: "{{ nodes.film.outputs.film_subtitles }}"
      seconds: "{{ nodes.film.outputs.film_seconds }}"
    parameters:
      output_format: json
      template: |
        {"deliverable": {
          "kind": "video_overview",
          "title": {{ plan.title | json_encode() }},
          "content": {"scenes": {{ plan.scenes | json_encode() }}, "seconds": {{ seconds | json_encode() }}},
          "citations": [{% set_global first = true %}{% for l in plan.scenes %}{% if l.chunk_id %}{% set_global sid = "" %}{% set_global passage = "" %}{% for c in chunks %}{% if c.chunk_id == l.chunk_id %}{% set_global sid = c.source_id %}{% set_global passage = c.content %}{% endif %}{% endfor %}{% if passage %}{% if not first %},{% endif %}{% set_global first = false %}{"source_id": {{ sid | json_encode() }}, "chunk_id": {{ l.chunk_id | json_encode() }}, "quote": {{ passage | truncate(length=300) | json_encode() }}}{% endif %}{% endif %}{% endfor %}],
          "files": [
            {"node_id": "film", "name": {{ mp4 | split(pat="/") | last | json_encode() }}, "mime_type": "video/mp4", "role": "primary"},
            {"node_id": "film", "name": {{ srt | split(pat="/") | last | json_encode() }}, "mime_type": "application/x-subrip", "role": "asset"}
          ]
        }}
`,i5=`# Daily Workflow 5 — \`video_summarizer\`: Video URL / Local Video File -> Native Subtitles or FFmpeg + ASR (.srt) -> Key Viewpoints & Timestamped Chapters + Mindmap + SRT Subtitles + PDF/HTML Summary.
#
#   yanshi workflow run examples/generators/video_summarizer.yml \\
#     --input source "https://www.youtube.com/watch?v=..." --output-dir out
name: video_summarizer
description: "输入在线视频链接或上传本地音视频文件，优先提取内封/平台字幕并自动回退 FFmpeg 音轨抽取 + ASR 语音转写生成标准 .srt 字幕，提炼核心观点、时间戳章节纪要、思维导图与精要 PDF"

inputs:
  source:
    description: "在线视频 URL（支持 yt-dlp 站点与直链 MP4/WebM）或本地视频/音频文件（支持 Studio 一键选择本地文件）"
    default: "https://media.w3.org/2010/05/sintel/trailer.mp4"
  subtitle_langs:
    description: "优先抓取的原生字幕语言列表（逗号分隔）"
    default: "zh-Hans,zh,en"
  asr_model:
    description: "无现成字幕时使用的语音识别模型（如 gemini-2.5-flash-asr、glm-asr、whisper-1）"
    default: "gemini-2.5-flash-asr"
  summary_lang:
    description: "观点总结与双语润色字幕的目标语言"
    default: "简体中文"
  polish_bilingual_srt:
    description: "是否额外生成一份按时间轴校对润色的目标语言 .srt 字幕文件"
    default: true
  export_pdf:
    description: "同时导出排版精美的视频总结报告 PDF"
    default: true

nodes:
  - id: ingest
    type: video_ingest
    parameters:
      prefer_subtitles: true
      max_duration_seconds: 5400

  - id: asr_opts
    type: template
    parameters:
      output_format: json
      template: '{"asr_model": {{ asr_model | json_encode() }}, "polish_srt": {{ polish_bilingual_srt }}, "want_pdf": {{ export_pdf }}}'

  - id: transcribe
    type: asr
    dependencies: ["ingest", "asr_opts"]
    input_mapping:
      model: "{{ nodes.asr_opts.outputs.asr_model }}"
      audio_source: "{{ nodes.ingest.outputs.media_or_subtitles }}"
    parameters:
      response_format: srt

  - id: summary_prompt
    type: template
    dependencies: ["ingest", "transcribe"]
    input_mapping:
      video_title: "{{ nodes.ingest.outputs.title }}"
      duration_secs: "{{ nodes.ingest.outputs.duration_seconds }}"
      raw_srt: "{{ nodes.transcribe.outputs.srt }}"
      plain_text: "{{ nodes.transcribe.outputs.text }}"
    parameters:
      template: |
        请针对视频《{{ video_title }}》（时长约 {{ duration_secs | default(value=0) | round }} 秒）的字幕与转写文稿，使用「{{ summary_lang }}」深度提炼结构化视频精要：
        1. \`video_title\`：规范化后的视频标题；
        2. \`executive_summary\`：200-350 字高密度全景摘要，交代核心主题、背景脉络与最终结论；
        3. \`key_viewpoints\`：提炼 4-8 条核心观点，每条包含 \`viewpoint\`、\`supporting_detail\`（具体论据/案例/数据）与 \`timestamp_hint\`（对应出现的大致时间点，如 \`01:25\`）；
        4. \`chapters\`：按时间推进划分的章节大纲（\`time_range\`、\`heading\`、\`summary\`）；
        5. \`memorable_quotes\`：视频中最具启发性的金句或关键结论原话精译；
        6. \`actionable_takeaways\`：面向听众的可执行启发与落地建议；
        7. \`mindmap_tree\`：概括全片脉络的思维导图树（根节点为视频主题，3 层以内分支，标签简明凝练）。

        带时间轴的原始 SRT 字幕：
        {{ raw_srt | truncate(length=42000) }}

        纯文本参考：
        {{ plain_text | truncate(length=12000) }}

  - id: summarize_video
    type: llm
    dependencies: ["summary_prompt"]
    input_mapping:
      prompt: "{{ nodes.summary_prompt.outputs.output }}"
    parameters:
      max_tokens: 8192
      system: "你是专业的内容主编与知识萃取专家，严格依据视频真实字幕内容提炼观点与时间点，仅输出合法 JSON。"
      json_schema:
        type: object
        required:
          - video_title
          - executive_summary
          - key_viewpoints
          - chapters
          - actionable_takeaways
          - mindmap_tree
        properties:
          video_title: { type: string }
          executive_summary: { type: string }
          key_viewpoints:
            type: array
            minItems: 1
            maxItems: 10
            items:
              type: object
              required: [viewpoint, supporting_detail, timestamp_hint]
              properties:
                viewpoint: { type: string }
                supporting_detail: { type: string }
                timestamp_hint: { type: string }
          chapters:
            type: array
            minItems: 1
            maxItems: 10
            items:
              type: object
              required: [time_range, heading, summary]
              properties:
                time_range: { type: string }
                heading: { type: string }
                summary: { type: string }
          memorable_quotes:
            type: array
            items: { type: string }
          actionable_takeaways:
            type: array
            items: { type: string }
          mindmap_tree:
            type: object
            required: [label, children]
            properties:
              label: { type: string }
              children:
                type: array
                items:
                  type: object
                  required: [label]
                  properties:
                    label: { type: string }
                    children:
                      type: array
                      items:
                        type: object
                        required: [label]
                        properties:
                          label: { type: string }
                          children:
                            type: array
                            items:
                              type: object
                              required: [label]
                              properties:
                                label: { type: string }

  - id: polish_srt_prompt
    type: template
    dependencies: ["transcribe", "asr_opts"]
    run_if: "{{ nodes.asr_opts.outputs.polish_srt }}"
    input_mapping:
      raw_srt: "{{ nodes.transcribe.outputs.srt }}"
    parameters:
      template: |
        请将以下原始 SRT 字幕校对、断句优化并翻译/润色为标准「{{ summary_lang }}」SRT 字幕格式（严格保持 \`序号\` + \`HH:MM:SS,mmm --> HH:MM:SS,mmm\` + \`字幕正文\` + 空行的标准 SubRip 格式，不要包裹代码块标记）：

        {{ raw_srt | truncate(length=28000) }}

  - id: polish_srt_llm
    type: llm
    dependencies: ["polish_srt_prompt", "asr_opts"]
    run_if: "{{ nodes.asr_opts.outputs.polish_srt }}"
    input_mapping:
      prompt: "{{ nodes.polish_srt_prompt.outputs.output }}"
    parameters:
      max_tokens: 8192
      system: "你只输出严格合法的标准 SubRip (.srt) 字幕纯文本，绝不输出任何额外解释或 Markdown 代码围栏。"

  - id: mindmap_tree_json
    type: template
    dependencies: ["summarize_video"]
    input_mapping:
      summary: "{{ nodes.summarize_video.outputs.output }}"
    parameters:
      output_format: json
      template: '{"tree": {{ summary.mindmap_tree | json_encode() }}}'

  - id: mindmap_svg
    type: render_mindmap
    dependencies: ["mindmap_tree_json"]
    input_mapping:
      tree: "{{ nodes.mindmap_tree_json.outputs.tree }}"

  - id: mindmap_png
    type: render_svg
    dependencies: ["mindmap_svg"]
    input_mapping:
      svg: "{{ nodes.mindmap_svg.outputs.mindmap_svg_output }}"
    parameters:
      width: 1600

  - id: mindmap_html
    type: render_mindmap
    dependencies: ["mindmap_tree_json"]
    input_mapping:
      tree: "{{ nodes.mindmap_tree_json.outputs.tree }}"
    parameters:
      format: html

  - id: assemble_report
    type: template
    dependencies: ["ingest", "transcribe", "summarize_video", "mindmap_png", "polish_srt_llm"]
    input_mapping:
      has_native_subs: "{{ nodes.ingest.outputs.has_subtitles }}"
      duration_secs: "{{ nodes.ingest.outputs.duration_seconds }}"
      summary: "{{ nodes.summarize_video.outputs.output }}"
      mm_png: "{{ nodes.mindmap_png.outputs.mindmap_png_output }}"
      polished_srt: "{{ nodes.polish_srt_llm.outputs.output }}"
      raw_srt: "{{ nodes.transcribe.outputs.srt }}"
    parameters:
      template: |
        # {{ summary.video_title }} — 视频核心观点与字幕精要

        > **时长**：约 {{ duration_secs | default(value=0) | round }} 秒 · **字幕来源**：{% if has_native_subs %}原生内嵌/平台字幕直提{% else %}FFmpeg 音轨抽取 + ASR 语音转写 (\`{{ asr_model }}\`){% endif %}

        ## 1. 全片核心摘要

        {{ summary.executive_summary }}

        ## 2. 知识脉络思维导图

        ![视频脉络思维导图]({{ mm_png }})

        ## 3. 核心观点与论据提炼

        | 时间点 | 核心观点 | 关键论据 / 细节支撑 |
        | --- | --- | --- |
        {% for v in summary.key_viewpoints %}| \`{{ v.timestamp_hint }}\` | **{{ v.viewpoint }}** | {{ v.supporting_detail }} |
        {% endfor %}

        ## 4. 时间轴章节纪要

        {% for ch in summary.chapters %}
        ### \`{{ ch.time_range }}\` {{ ch.heading }}

        {{ ch.summary }}
        {% endfor %}

        {% if summary.memorable_quotes and summary.memorable_quotes | length > 0 %}
        ## 5. 关键金句摘录

        {% for q in summary.memorable_quotes %}> "{{ q }}"

        {% endfor %}
        {% endif %}

        {% if summary.actionable_takeaways and summary.actionable_takeaways | length > 0 %}
        ## 6. 实践启发与行动清单

        {% for a in summary.actionable_takeaways %}- {{ a }}
        {% endfor %}
        {% endif %}

        ## 7. 完整时间轴字幕（SRT）

        \`\`\`srt
        {% if polished_srt %}{{ polished_srt }}{% else %}{{ raw_srt }}{% endif %}
        \`\`\`

  - id: summary_md
    type: render_markdown
    dependencies: ["assemble_report"]
    input_mapping:
      markdown: "{{ nodes.assemble_report.outputs.output }}"
    parameters:
      format: md

  - id: summary_html
    type: render_markdown
    dependencies: ["assemble_report"]
    input_mapping:
      markdown: "{{ nodes.assemble_report.outputs.output }}"

  - id: summary_pdf
    type: render_document
    dependencies: ["assemble_report", "asr_opts"]
    run_if: "{{ nodes.asr_opts.outputs.want_pdf }}"
    input_mapping:
      markdown: "{{ nodes.assemble_report.outputs.output }}"
    parameters:
      format: pdf
      toc: true

  - id: deliverable
    type: template
    dependencies:
      - transcribe
      - summarize_video
      - mindmap_svg
      - mindmap_html
      - summary_md
      - summary_html
      - summary_pdf
    input_mapping:
      srt_path: "{{ nodes.transcribe.outputs.srt_file }}"
      summary: "{{ nodes.summarize_video.outputs.output }}"
      mm_svg: "{{ nodes.mindmap_svg.outputs.mindmap_svg_output }}"
      mm_html: "{{ nodes.mindmap_html.outputs.mindmap_html_output }}"
      md: "{{ nodes.summary_md.outputs.summary_md_output }}"
      html: "{{ nodes.summary_html.outputs.summary_html_output }}"
      pdf: "{{ nodes.summary_pdf.outputs.summary_pdf_output }}"
    parameters:
      output_format: json
      template: |
        {"deliverable": {
          "kind": "video_summarizer",
          "title": {{ summary.video_title | json_encode() }},
          "content": {{ summary | json_encode() }},
          "citations": [],
          "files": [
            {"node_id": "transcribe", "name": {{ srt_path | split(pat="/") | last | json_encode() }}, "mime_type": "application/x-subrip", "role": "primary"},
            {"node_id": "summary_html", "name": {{ html | split(pat="/") | last | json_encode() }}, "mime_type": "text/html", "role": "preview"},
            {"node_id": "summary_md", "name": {{ md | split(pat="/") | last | json_encode() }}, "mime_type": "text/markdown", "role": "asset"},
            {"node_id": "mindmap_svg", "name": {{ mm_svg | split(pat="/") | last | json_encode() }}, "mime_type": "image/svg+xml", "role": "asset"},
            {"node_id": "mindmap_html", "name": {{ mm_html | split(pat="/") | last | json_encode() }}, "mime_type": "text/html", "role": "preview"}{% if pdf %},
            {"node_id": "summary_pdf", "name": {{ pdf | split(pat="/") | last | json_encode() }}, "mime_type": "application/pdf", "role": "primary"}{% endif %}
          ]
        }}
`,jg="studio.templates.index",a5="studio.template.",o5=15*1024;function Ng(e){return`${a5}${e}`}const r5=[{id:"illustrated_slides",title:"主题调研与精美配图 PPT (Illustrated Slides)",subtitle:"联网检索 → 规划大纲 → 艺术总监精炼配图 + D2 架构图 → 原生 OOXML 可编辑 PPTX + 16:9 PDF + HTML",badge:"日常生产力",category:"课件与视频",yaml:G6},{id:"repo_architecture",title:"源码 / GitHub 仓库架构与依赖图 (Repo Architecture)",subtitle:"自动扫描代码树、Manifest 与依赖边 → D2 系统分层架构图 + 模块依赖 DAG 图 + 深度报告",badge:"日常生产力",category:"导图与工程图",yaml:W6},{id:"backend_project_planner",title:"多轮对话后端项目全套规划 (Backend Planner)",subtitle:"多轮交互澄清需求 → 系统架构图 / 模块图 / UML 类图与时序图 / 流程图 / ER 图 + PG DDL + 导图",badge:"日常生产力",category:"导图与工程图",yaml:Y6},{id:"paper_reader",title:"学术论文双语精读与实验数据提取 (Paper Reader)",subtitle:"支持 PDF 文件或 arXiv 链接 → 双语精译 + 核心思想论点 + 实验数据表格 XLSX/CSV + 导图 + PDF",badge:"日常生产力",category:"研究报告与信息图",yaml:J6},{id:"video_summarizer",title:"视频观点提炼与字幕导图生成 (Video Summarizer)",subtitle:"支持视频 URL 或本地文件 → 原生字幕直提或 FFmpeg+ASR 生成 .srt → 时间轴观点精要 + 思维导图",badge:"日常生产力",category:"课件与视频",yaml:i5},{id:"tech_slides_video",title:"拟真人 PPT 演播级技术精讲视频 (Keynote Presenter Video)",subtitle:"多源支撑 → 视线引导口播 + 精美插图/D2 架构图 → 按步配音三模式 1080p 视频 + 原生可编辑 PPTX / 16:9 PDF",badge:"旗舰工作流",category:"课件与视频",yaml:n5},{id:"slides",title:"通用主题演示课件 (Slides)",subtitle:"来源资料自动提炼大纲、结构化卡片与矢量图表，直出原生可编辑 OOXML PPTX 与 HTML 放映页",badge:"官方生成器",category:"课件与视频",yaml:t5},{id:"video_overview",title:"多镜头视频概览 (Video Overview)",subtitle:"自动编排分镜脚本、视觉画面、配音旁白与字幕合成完整 MP4 解说视频",badge:"官方生成器",category:"课件与视频",yaml:s5},{id:"report",title:"深度溯源研究报告 (Report)",subtitle:"大纲规划 → 多章节并行撰写与原文引用脚注 → 导出 Markdown / HTML / PDF",badge:"官方生成器",category:"研究报告与信息图",yaml:e5},{id:"infographic",title:"结构化信息长图 (Infographic)",subtitle:"提炼核心指标与分栏对比结论，排版渲染高清 SVG 与 PNG 视觉信息长图",badge:"官方生成器",category:"研究报告与信息图",yaml:X6},{id:"mindmap",title:"知识全景思维导图 (Mindmap)",subtitle:"从网页或长文本抽取多层概念树，输出矢量 SVG、PNG 与可交互缩放 Markmap HTML",badge:"官方生成器",category:"导图与工程图",yaml:F6},{id:"circuit_diagram",title:"工程电路原理图 (Circuit Diagram)",subtitle:"元器件清单抽取 + CircuiTikZ 编译闭环自检，输出工程级 SVG 与 PNG 原理图",badge:"工程制图",category:"导图与工程图",yaml:V6},{id:"audio_overview",title:"双人播客对谈 (Audio Overview)",subtitle:"主持人与嘉宾自然问答脚本生成 + 多音色并行合成 + FFmpeg 混音导出播客音频",badge:"官方生成器",category:"播客音频",yaml:P6},{id:"flashcards",title:"互动翻转记忆闪卡 (Flashcards)",subtitle:"围绕核心概念生成问答卡片集与免依赖点击翻转复习 HTML 单页",badge:"互动学习",category:"互动学习",yaml:K6},{id:"quiz",title:"即时反馈自测题库 (Quiz)",subtitle:"自动生成多选题、干扰项与答案解析，输出可交互判分 HTML 测验页",badge:"互动学习",category:"互动学习",yaml:Q6}];function l5(e){const t=(e??"").trim();return t?/^[a-zA-Z0-9_.-]+\.(yml|yaml)$/i.test(t)?t:`${t.toLowerCase().replace(/\.(yml|yaml)$/i,"").replace(/[^a-z0-9_-]+/g,"_").replace(/^_+|_+$/g,"")||"workflow"}.yml`:"workflow.yml"}function c5(e){const t=e.toLowerCase().replace(/[^a-z0-9_-]+/g,"_").replace(/^_+|_+$/g,"").slice(0,32),s=Math.random().toString(36).slice(2,8);return t?`${t}_${s}`:`tpl_${s}`}function u5(e){return new TextEncoder().encode(e).length}function d5(e,t,s=new Date().toISOString()){const a=t.title.trim()||"未命名个人模板",o=t.yaml;if(!o.trim())throw new Error("工作流 YAML 内容不能为空");if(u5(JSON.stringify({yaml:o}))>o5)throw new Error("单个收藏模板大小不能超过 15 KiB，请导出为本地 .yml 文件保存");const c=t.id?.trim()||c5(a),f={id:c,title:a,yaml:o,updatedAt:s},m=e.filter(y=>y.id!==c),p=[f,...m].slice(0,24),_=p.map(y=>({id:y.id,title:y.title,updatedAt:y.updatedAt}));return{nextTemplates:p,preferencePatch:{[jg]:{items:_},[Ng(c)]:{yaml:o}}}}function f5(e,t){const s=e.filter(o=>o.id!==t),a=s.map(o=>({id:o.id,title:o.title,updatedAt:o.updatedAt}));return{nextTemplates:s,preferencePatch:{[jg]:{items:a},[Ng(t)]:{yaml:""}}}}function p5(e){if(!e||typeof e!="object")return[];const t=e[jg];if(!t||typeof t!="object")return[];const s=t.items;if(!Array.isArray(s))return[];const a=[];for(const o of s){if(!o||typeof o!="object")continue;const l=o,c=typeof l.id=="string"?l.id.trim():"",f=typeof l.title=="string"?l.title.trim():"",m=typeof l.updatedAt=="string"?l.updatedAt:"";if(!c||!f)continue;const p=e[Ng(c)],_=p&&typeof p=="object"&&typeof p.yaml=="string"?p.yaml:"";_.trim()&&a.push({id:c,title:f,yaml:_,updatedAt:m})}return a}const m5=`name: instant_step_debug_demo
inputs:
  topic:
    description: "演练主题（免 API Key 本地秒级单步观测）"
    default: "Rust 所有权与借用检查器"
  difficulty:
    description: "目标深度（入门 / 进阶 / 架构级）"
    default: "架构级"
  include_quiz:
    description: "是否生成随堂思考题分支（触发 run_if 条件边）"
    default: true

nodes:
  - id: normalize_brief
    type: template
    parameters:
      output_format: json
      template: |
        {
          "headline": {{ topic | json_encode() }},
          "level": {{ difficulty | json_encode() }},
          "quiz_enabled": {{ include_quiz }}
        }

  - id: core_outline
    type: template
    dependencies: ["normalize_brief"]
    input_mapping:
      headline: "{{ nodes.normalize_brief.outputs.headline }}"
      level: "{{ nodes.normalize_brief.outputs.level }}"
    parameters:
      template: |
        ## {{ headline }}（{{ level }}）
        1. 核心心智模型与不变式
        2. 编译器静态消解 vs 运行期开销对比
        3. 工程边界与常见反模式

  - id: quiz_branch
    type: template
    dependencies: ["normalize_brief"]
    run_if: "{{ nodes.normalize_brief.outputs.quiz_enabled }}"
    input_mapping:
      headline: "{{ nodes.normalize_brief.outputs.headline }}"
    parameters:
      template: |
        ### 随堂自测（{{ headline }}）
        - 为什么可变引用在同一时刻具有独占别名语义？
        - 何时应当把生命周期上界收拢到方法签名而非结构体定义？

  - id: assemble_handout
    type: template
    dependencies: ["core_outline", "quiz_branch"]
    input_mapping:
      outline: "{{ nodes.core_outline.outputs.output }}"
      quiz: "{{ nodes.quiz_branch.outputs.output }}"
    parameters:
      template: |
        # 课程讲义汇总

        {{ outline }}

        {% if quiz is defined %}{{ quiz }}{% else %}_（已跳过随堂思考题分支）_{% endif %}
`,h5=`name: instant_slide_render_demo
inputs:
  deck_title:
    description: "幻灯片主标题（免 LLM 直接验证 SVG/PNG 课件渲染与预览）"
    default: "Rust Trait 分发机制可视化"
  slide_theme:
    description: "配色主题：dark 或 light"
    default: "dark"

nodes:
  - id: sample_deck
    type: template
    parameters:
      output_format: json
      template: |
        {
          "deck": {
            "title": {{ deck_title | json_encode() }},
            "subtitle": "静态单态化 vs vtable 胖指针内存模型",
            "author": "Yanshi Studio",
            "theme": {{ slide_theme | json_encode() }},
            "slides": [
              {
                "section": "01 · 核心抽象",
                "title": "用行为契约替代继承层级",
                "left": {
                  "kind": "code",
                  "language": "rust",
                  "title": "trait Summary",
                  "code": "pub trait Summary {\\n  fn summarize(&self) -> String;\\n}\\n\\npub fn notify(item: &impl Summary) {\\n  println!(\\"Breaking: {}\\", item.summarize());\\n}",
                  "callouts": [{"line": 5, "text": "静态分发：编译期单态化零虚表开销", "tone": "rust"}]
                },
                "right": {
                  "kind": "table",
                  "columns": ["维度", "Rust Trait", "OOP 继承", "C 函数指针"],
                  "highlight_col": 1,
                  "rows": [
                    ["编译期类型安全", "✓ 强约束", "✓ 强约束", "✗ void* 易出错"],
                    ["运行期零开销", "✓ 单态化内联", "✗ 虚表跳转", "✗ 间接调用"],
                    ["为外部类型扩展", "✓ 孤儿规则内支持", "✗ 需改基类", "⚠ 手工封装"]
                  ]
                },
                "takeaway": "优先使用泛型静态分发，仅在异构集合边界按需退化到 &dyn Trait。",
                "notes": "我们先看 Trait 的声明方式。编译器通过单态化把泛型调用直接内联。右边对比了三种抽象方案的取舍。"
              }
            ]
          }
        }

  - id: render_preview
    type: render_slides
    dependencies: ["sample_deck"]
    input_mapping:
      deck: "{{ nodes.sample_deck.outputs.deck }}"
    parameters:
      width: 1920
      progressive: true
`,nr=[...r5,{id:"instant_slide_render_demo",title:"免 Key 幻灯片秒级渲染演练",subtitle:"2 节点直出 1080p 代码/对比矩阵幻灯片与交互式 HTML，立即可观测图像预览",badge:"秒级图像观测",category:"免 Key 演练",yaml:h5},{id:"instant_step_debug_demo",title:"免 Key 单步与条件分支演练",subtitle:"4 节点含 run_if 条件边，零外部依赖体验每步暂停核查输入/输出",badge:"调试入门",category:"免 Key 演练",yaml:m5}];function g5(e){return typeof e=="boolean"?"boolean":typeof e=="number"?"number":Array.isArray(e)||e!==null&&typeof e=="object"?"json":"string"}function y5(e){const s=e.trim().replace(/^\{\{/,"").replace(/\}\}$/,"").trim().split(".");return s.length===4&&s[0]==="nodes"&&s[2]==="outputs"?{source:s[1],output:s[3]}:null}function _5(e){const t=[],s=/(?:^|[^a-zA-Z0-9_-])nodes\.([a-zA-Z0-9_-]+)/g;let a=s.exec(e);for(;a;){const o=a[1];o&&!t.includes(o)&&t.push(o),a=s.exec(e)}return t}function Cx(e){let t;try{t=kS.parse(e)}catch(E){const O=E instanceof Error?E.message:String(E);return{valid:!1,name:null,cost_limit_usd:null,inputs:[],nodes:[],edges:[],execution_order:[],levels:[],diagnostics:{valid:!1,issues:[`YAML 语法错误: ${O}`],warnings:[]}}}if(!t||typeof t!="object"||Array.isArray(t))return{valid:!1,name:null,cost_limit_usd:null,inputs:[],nodes:[],edges:[],execution_order:[],levels:[],diagnostics:{valid:!1,issues:["工作流顶层必须是 YAML 对象（包含 name 与 nodes）"],warnings:[]}};const s=t,a=typeof s.name=="string"?s.name:null,o=typeof s.cost_limit_usd=="number"?s.cost_limit_usd:null,l=s.inputs&&typeof s.inputs=="object"&&!Array.isArray(s.inputs)?s.inputs:{},c=Object.entries(l).map(([E,O])=>({name:E,description:typeof O?.description=="string"?O.description:null,required:!!O?.required,default:O?.default,kind:g5(O?.default)})).sort((E,O)=>Number(O.required)-Number(E.required)||E.name.localeCompare(O.name)),f=Array.isArray(s.nodes)?s.nodes:[],m=new Set;for(const E of f)E&&typeof E=="object"&&typeof E.id=="string"&&m.add(E.id);const p=[],_=[];for(const E of f){if(!E||typeof E!="object")continue;const O=E,R=typeof O.id=="string"?O.id:"",q=typeof O.type=="string"?O.type:"unknown";if(!R)continue;const Z=Array.isArray(O.dependencies)?O.dependencies.filter(U=>typeof U=="string"):[],oe={};if(O.input_mapping&&typeof O.input_mapping=="object"&&!Array.isArray(O.input_mapping))for(const[U,B]of Object.entries(O.input_mapping))typeof B=="string"&&(oe[U]=B);const Q=typeof O.run_if=="string"?O.run_if:null,ee=new Set,X=new Map;for(const[U,B]of Object.entries(oe).sort(([V],[te])=>V.localeCompare(te))){const V=y5(B);if(V&&V.source!==R&&m.has(V.source)){const te=X.get(V.source)??[];te.push(`${V.output} → ${U}`),X.set(V.source,te)}}for(const[U,B]of X.entries())ee.add(U),p.push({from:U,to:R,kind:"input_mapping",label:B.join(", ")});if(Q)for(const U of _5(Q))U!==R&&m.has(U)&&(ee.add(U),p.push({from:U,to:R,kind:"condition",label:"run_if"}));for(const U of Z)U!==R&&m.has(U)&&!ee.has(U)&&(ee.add(U),p.push({from:U,to:R,kind:"dependency",label:null}));const L=O.parameters&&typeof O.parameters=="object"&&!Array.isArray(O.parameters)?{...O.parameters}:{};let I=[];const D=q==="map"?"template":q==="while"?"do":null;D&&Array.isArray(L[D])&&(I=L[D].filter(U=>U&&typeof U.id=="string").map((U,B)=>({id:String(U.id),type:typeof U.type=="string"?U.type:"unknown",dependencies:[],input_mapping:{},parameters:U.parameters??{},level:B})),delete L[D]),_.push({id:R,type:q,dependencies:Z,input_mapping:oe,run_if:Q,timeout_ms:typeof O.timeout_ms=="number"?O.timeout_ms:null,max_retries:typeof O.max_retries=="number"?O.max_retries:null,parameters:L,sub_nodes:I})}const y=new Map,b=new Map,v=new Map;for(const E of _)y.set(E.id,0),b.set(E.id,new Set),v.set(E.id,0);for(const E of p){const O=b.get(E.from);O&&!O.has(E.to)&&(O.add(E.to),y.set(E.to,(y.get(E.to)??0)+1))}const S=[...y.entries()].filter(([,E])=>E===0).map(([E])=>E).sort(),w=[];for(;S.length>0;){const E=S.shift();w.push(E);const O=v.get(E)??0,R=[...b.get(E)??[]].sort();for(const q of R){v.set(q,Math.max(v.get(q)??0,O+1));const Z=(y.get(q)??1)-1;y.set(q,Z),Z===0&&S.push(q)}}const k=[];_.length===0&&k.push("工作流至少需要定义 1 个节点"),w.length!==_.length&&k.push("检测到节点循环依赖（DAG 必须无环）");const j=_.map(E=>({...E,level:v.get(E.id)??0})),A=j.reduce((E,O)=>Math.max(E,O.level),0),N=w.length>0?Array.from({length:A+1},()=>[]):[];for(const E of w)N[v.get(E)??0]?.push(E);const T=k.length===0;return{valid:T,name:a,cost_limit_usd:o,inputs:c,nodes:j,edges:p,execution_order:w,levels:N,diagnostics:{valid:T,issues:k,warnings:[]}}}function Wu(e){if(e.default===void 0||e.default===null)return e.kind==="boolean"?"false":e.kind==="json"?"{}":"";if(e.kind==="json")try{return JSON.stringify(e.default,null,2)}catch{return String(e.default)}return String(e.default)}function b5(e,t){const s={};for(const a of e){const o=t[a.name]??Wu(a);if(a.kind==="boolean")s[a.name]=o==="true";else if(a.kind==="number"){const l=o.trim();if(!l){if(a.required)return{payload:s,error:`必填数值参数「${a.name}」不能为空`};continue}const c=Number(l);if(Number.isNaN(c))return{payload:s,error:`参数「${a.name}」需要合法数字，当前为 "${o}"`};s[a.name]=c}else if(a.kind==="json"){const l=o.trim();if(!l)continue;try{s[a.name]=JSON.parse(l)}catch(c){const f=c instanceof Error?c.message:String(c);return{payload:s,error:`参数「${a.name}」JSON 解析失败：${f}`}}}else{if(a.required&&!o.trim())return{payload:s,error:`必填参数「${a.name}」不能为空`};s[a.name]=o}}return{payload:s,error:null}}function Xa(e){return!Number.isFinite(e)||e<=0?"$0.00":e<.01?`$${e.toFixed(4)}`:e<1?`$${e.toFixed(3)}`:`$${e.toFixed(2)}`}function Nk(e){const t={},s={},a={};let o=null,l=null,c=null,f=0,m=null;const p=_=>(t[_]||(t[_]={status:"idle"}),t[_]);for(const _ of e){const y=_.payload??{},b=typeof y.node_id=="string"?y.node_id:null;switch(_.kind){case"workflow.started":case"checkpoint.restored":{l=null,m=null;break}case"node.started":{if(b){const v=p(b);v.status="running",v.error=void 0}break}case"node.input.captured":{if(b){s[b]=y.input;const v=p(b);v.hasInput=!0}break}case"node.output.captured":{if(b){a[b]=y.output;const v=p(b);v.hasOutput=!0}break}case"node.completed":{if(b){const v=p(b);v.status="succeeded",typeof y.duration_ms=="number"&&(v.durationMs=y.duration_ms)}break}case"node.cost":{if(b&&typeof y.cost_usd=="number"&&Number.isFinite(y.cost_usd)){const v=p(b);v.costUsd=y.cost_usd}typeof y.total_cost_usd=="number"&&Number.isFinite(y.total_cost_usd)?f=Math.max(f,y.total_cost_usd):typeof y.cost_usd=="number"&&Number.isFinite(y.cost_usd)&&(f+=y.cost_usd);break}case"cost.limit_exceeded":{const v=typeof y.spent_usd=="number"&&Number.isFinite(y.spent_usd)?y.spent_usd:f,S=typeof y.limit_usd=="number"&&Number.isFinite(y.limit_usd)?y.limit_usd:0;f=Math.max(f,v),m={nodeId:b??"unknown",spentUsd:v,limitUsd:S};break}case"node.skipped":{if(b){const v=p(b);v.status="skipped",typeof y.reason=="string"&&(v.skipReason=y.reason)}break}case"node.failed":{if(b){const v=p(b);v.status="failed",typeof y.error=="string"&&(v.error=y.error),typeof y.duration_ms=="number"&&(v.durationMs=y.duration_ms)}break}case"node.debug.paused":{if(b){const v=y.phase==="before_node"?"before_node":"after_node",S=Array.isArray(y.next_nodes)?y.next_nodes.filter(k=>typeof k=="string"):[],w=p(b);w.status=v==="before_node"?"paused_before":"paused_after",o={node_id:b,phase:v,next_nodes:S}}break}case"node.debug.resumed":{if(b){const v=p(b);v.status=v.status==="paused_before"?"running":"succeeded",o?.node_id===b&&(o=null)}else o=null;break}case"workflow.completed":{o=null,l=null,typeof y.duration_ms=="number"&&(c=y.duration_ms);break}case"workflow.failed":{o=null,typeof y.error=="string"&&(l=y.error),typeof y.duration_ms=="number"&&(c=y.duration_ms);break}case"workflow.cancelled":{o=null;break}}}return{telemetryByNode:t,nodeInputs:s,nodeOutputs:a,pausedAt:o,workflowError:l,workflowDurationMs:c,totalCostUsd:f,costLimitExceeded:m}}function Vm({initialMode:e,apiToken:t,onTokenChange:s}){const a=Uh(),[o,l]=C.useState(()=>e||(bt(Bv,"debug")==="release"?"release":"debug")),[c,f]=C.useState(()=>bt(gm,nr[0].id)),[m,p]=C.useState(()=>{const $=bt(gm,nr[0].id);return(nr.find(pe=>pe.id===$)??nr[0]).yaml}),[_,y]=C.useState(()=>Cx(m)),[b,v]=C.useState("local"),[S,w]=C.useState({}),[k,j]=C.useState(()=>bt(Ni,"default")),[A,N]=C.useState(()=>bt(ad,"")),[T,E]=C.useState("全部"),[O,R]=C.useState(()=>{const $=bt(Eu,"");if(!$)return[];try{const W=JSON.parse($);return Array.isArray(W)?W:[]}catch{return[]}}),[q,Z]=C.useState(!1),[oe,Q]=C.useState(""),[ee,X]=C.useState(!1),[L,I]=C.useState(null),[D,U]=C.useState(()=>bt(Lh,"")),[B,V]=C.useState(!1),[te,z]=C.useState(!1),[Y,ce]=C.useState(!1),[de,_e]=C.useState(!0),[F,G]=C.useState(!1),[P,ae]=C.useState("canvas"),[Se,je]=C.useState(!0),[we,ve]=C.useState([]),[xe,ke]=C.useState(null),[me,Ge]=C.useState(""),[Ke,rt]=C.useState(null),[nt,qe]=C.useState([]),[Xt,Pt]=C.useState([]),[lt,St]=C.useState([]),[mn,Tt]=C.useState(null),[Ft,Jt]=C.useState("idle"),[zn,zi]=C.useState(!1),[Us,Dn]=C.useState(!1),[Qt,Be]=C.useState(null),[Mt,Bs]=C.useState(""),[va,le]=C.useState(!1),[Te,Re]=C.useState(!1),[Ne,Ce]=C.useState(null),[ct,wt]=C.useState(null),[Vn,Ws]=C.useState(!1),[vs,Nn]=C.useState([]),[Hs,ei]=C.useState(null),[xa,ti]=C.useState([]),[Di,ni]=C.useState({}),[wa,Sa]=C.useState(null),En=C.useRef(null),kt=C.useRef(-1),Yt=C.useRef(0),as=C.useRef("");C.useEffect(()=>{if(!a.search)return;const $=new URLSearchParams(a.search),W=$.get("preset")?.trim();if(W){const be=nr.find(Oe=>Oe.id===W);be&&(f(be.id),p(be.yaml))}const pe=($.get("run_id")??$.get("run"))?.trim();pe&&pe!==me&&(Ge(pe),Jt("loading"))},[a.search]),C.useEffect(()=>{w($=>{const W={...$};let pe=!1;for(const be of _.inputs)be.name in W||(W[be.name]=Wu(be),pe=!0);return pe?W:$}),ke($=>$&&_.nodes.some(W=>W.id===$)?$:_.execution_order[0]??_.nodes[0]?.id??null)},[_]),C.useEffect(()=>{vt(Bv,o)},[o]),C.useEffect(()=>{vt(Ni,k)},[k]),C.useEffect(()=>{vt(ad,A)},[A]),C.useEffect(()=>{let $=!1;return(async()=>{try{const pe=await Me("/v1/preferences",t,{},k);if(!pe.ok||$)return;const be=await pe.json(),Oe=p5(be.preferences);$||(R(Oe),vt(Eu,JSON.stringify(Oe)))}catch{}})(),()=>{$=!0}},[t,k,D]),C.useEffect(()=>{const $=Cx(m);y($),v("local");let W=!1;const pe=window.setTimeout(async()=>{try{const be=await Me("/v1/workflows/inspect",t,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({workflow:m})},k);if(!be.ok||W){be.status===401&&!W&&z(!0);return}const Oe=await tt(HA,be,"POST /v1/workflows/inspect");W||(z(!1),y(Oe),v("server"))}catch{}},260);return()=>{W=!0,window.clearTimeout(pe)}},[m,t,k,D]);const xs=C.useCallback(async()=>{try{const $=await Me(`/v1/runs?tenant_id=${encodeURIComponent(k.trim()||"default")}&limit=10`,t,{},k);if(!$.ok){$.status===401&&z(!0);return}z(!1);const W=await tt(zh,$,"GET /v1/runs");qe(W.runs)}catch{}},[t,k]);C.useEffect(()=>{xs()},[xs,D]);const $i=C.useCallback(async $=>{if($)try{const W=await Me(`/v1/runs/${encodeURIComponent($)}/artifacts`,t,{},k);if(!W.ok)return;const pe=await tt(YA,W,`GET /v1/runs/${$}/artifacts`);St(pe.artifacts)}catch{}},[t,k]);C.useEffect(()=>{if(!me||Ft!=="loading")return;let $=!1,W;En.current?.abort(),kt.current=-1,Yt.current=0;const pe=Ve=>{Pt(yt=>yt.some(on=>on.seq===Ve.seq)?yt:[...yt,Ve].sort((on,ks)=>on.seq-ks.seq)),Ve.seq>kt.current&&(kt.current=Ve.seq),Ve.kind==="node.debug.paused"&&Tt(null)},be=async Ve=>{const yt=new AbortController;En.current=yt;const on=await Me(`/v1/runs/${encodeURIComponent(me)}/events?after_seq=${encodeURIComponent(String(Ve))}`,t,{signal:yt.signal},k);if(!on.ok||!on.body)throw new Error(`SSE 事件流连接失败 (HTTP ${on.status})`);Jt("streaming"),Yt.current=0;const ks=on.body.pipeThrough(new TextDecoderStream).getReader();let oi="";for(;!$;){const{done:Ht,value:Kn}=await ks.read();if(Ht)break;oi+=Kn;const So=Md(oi,Od,"studio SSE");oi=So.rest;for(const Oa of So.events)pe(Oa)}},Oe=()=>{if($)return;const Ve=Yt.current;Yt.current=Ve+1,Jt("reconnecting"),W=window.setTimeout(()=>{$||be(kt.current).catch(yt=>{$||yt instanceof DOMException&&yt.name==="AbortError"||(Be(yt instanceof Error?yt.message:String(yt)),Oe())})},Ew(Ve))};return(async()=>{try{const Ve=await Me(`/v1/runs/${encodeURIComponent(me)}`,t,{},k);if(!Ve.ok)throw new Error(`查询运行记录失败 (HTTP ${Ve.status})`);const yt=await tt(Ml,Ve,`GET /v1/runs/${me}`);if($)return;const on=dr(yt);rt(on),on.workflow&&on.workflow.trim()&&p(on.workflow),Pt([]),St([]),Be(null),new URLSearchParams(a.search).get("run")!==me&&gs(`${a.pathname}?run=${encodeURIComponent(me)}`,{replace:!0}),Me(`/v1/runs/${encodeURIComponent(me)}/debug`,t,{},k).then(Ht=>Ht.ok?tt(qv,Ht,"GET debug"):null).then(Ht=>{!$&&Ht?.active&&Ht.snapshot&&(ve(Ht.snapshot.breakpoints),je(Ht.snapshot.step_every_node),Tt(Ht.snapshot.paused_at??null))}).catch(()=>{});let ks=-1;const oi=await Me(`/v1/runs/${encodeURIComponent(me)}/events/history`,t,{},k);if(oi.ok){const Ht=await tt($h,oi,"GET events/history");$||(Pt(Ht),ks=Ht.at(-1)?.seq??-1,kt.current=ks)}if(cd(on)){Jt("closed"),await $i(me);return}if(await be(ks),!$){Jt("closed"),Tt(null);const Ht=await Me(`/v1/runs/${encodeURIComponent(me)}`,t,{},k);if(Ht.ok){const Kn=await tt(Ml,Ht,"GET final run");rt(dr(Kn))}await $i(me),xs()}}catch(Ve){if($||Ve instanceof DOMException&&Ve.name==="AbortError")return;Be(Ve instanceof Error?Ve.message:String(Ve)),Oe()}})(),()=>{$=!0,En.current?.abort(),W&&window.clearTimeout(W)}},[me,Ft,t,k,$i,xs]);const Je=C.useMemo(()=>Nk(Xt),[Xt]),hn=Je.pausedAt??mn;C.useEffect(()=>{if(!hn)return;const $=`${me}:${hn.node_id}:${hn.phase}:${Xt.length}`;as.current!==$&&(as.current=$,ke(hn.node_id))},[hn,me,Xt.length]);const gn=C.useMemo(()=>_.nodes.find($=>$.id===xe)??null,[_.nodes,xe]),yn=C.useMemo(()=>me?QO(me,lt,Je.nodeOutputs):[],[me,lt,Je.nodeOutputs]),un=C.useMemo(()=>{if(_.execution_order.length===0)return null;for(let $=_.execution_order.length-1;$>=0;$-=1){const W=_.execution_order[$],pe=Je.nodeOutputs[W];if(!pe||typeof pe!="object")continue;const be=pe;if(typeof be.output=="string"&&be.output.trim())return{nodeId:W,text:be.output}}return null},[_.execution_order,Je.nodeOutputs]),Tn=C.useMemo(()=>_.nodes.filter($=>{const W=Je.telemetryByNode[$.id]?.status;return W==="succeeded"||W==="skipped"||W==="paused_after"}).length,[_.nodes,Je.telemetryByNode]),zt=!!(me&&(Ft==="loading"||Ft==="streaming"||Ft==="reconnecting")&&(!Ke||!cd(Ke))),Cn=C.useMemo(()=>{const $=Mt.trim(),W=$&&Number.isFinite(Number($))&&Number($)>0?Number($):null,pe=typeof _.cost_limit_usd=="number"&&Number.isFinite(_.cost_limit_usd)&&_.cost_limit_usd>0?_.cost_limit_usd:null;return W!==null&&pe!==null?Math.min(W,pe):W??pe},[Mt,_.cost_limit_usd]),Dt=!!(me&&!zt&&(Ke?.status==="failed"||Ke?.status==="cancelled"||Je.workflowError));C.useEffect(()=>{if(!me||!zt){Nn([]);return}let $=!1;const W=async()=>{try{const be=await Me(`/v1/runs/${encodeURIComponent(me)}/approvals`,t,{},k);if(!be.ok||$)return;const Oe=await tt(XA,be,`GET /v1/runs/${me}/approvals`);$||Nn(Oe.approvals)}catch{}};W();const pe=window.setInterval(()=>{W()},2400);return()=>{$=!0,window.clearInterval(pe)}},[me,zt,t,k]);const bo=C.useMemo(()=>{for(let $=Xt.length-1;$>=0;$-=1){const W=Xt[$];if(W.kind==="node.progress"&&W.payload&&typeof W.payload=="object"&&W.payload.kind==="question.pending")return W.seq}return-1},[Xt]);C.useEffect(()=>{if(!me||!zt){ti([]);return}let $=!1;const W=async()=>{try{const be=await Me(`/v1/runs/${encodeURIComponent(me)}/questions`,t,{},k);if(!be.ok||$)return;const Oe=await tt(QA,be,`GET /v1/runs/${me}/questions`);$||ti(Oe.pending)}catch{}};W();const pe=window.setInterval(()=>{W()},1800);return()=>{$=!0,window.clearInterval(pe)}},[me,zt,t,k,bo]);const ws=C.useMemo(()=>[...O.map($=>({id:`personal:${$.id}`,title:$.title,subtitle:`个人收藏工作流 · 更新于 ${Zn($.updatedAt)}`,badge:"个人收藏",category:"个人收藏",yaml:$.yaml,personalId:$.id})),...nr.map($=>({...$,personalId:void 0}))],[O]),os=C.useMemo(()=>T==="全部"?ws:ws.filter($=>$.category===T),[ws,T]),rs=$=>{l($);const W=$==="release"?"/ui/app":"/ui/studio",pe=me?`?run=${encodeURIComponent(me)}`:"";gs(`${W}${pe}`,{replace:!0}),ae($==="release"?me?"deliverables":"config":"canvas")},vo=$=>{const W=ws.find(pe=>pe.id===$);W&&(f(W.id),vt(gm,W.id),p(W.yaml),w({}),ve([]))},ka=async $=>{const W=we.includes($)?we.filter(pe=>pe!==$):[...we,$];if(ve(W),me&&zt)try{await Me(`/v1/runs/${encodeURIComponent(me)}:debug`,t,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({action:"set_breakpoints",breakpoints:W,step_every_node:Se})},k)}catch{}},ja=async $=>{if(je($),me&&zt)try{await Me(`/v1/runs/${encodeURIComponent(me)}:debug`,t,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({action:"set_breakpoints",breakpoints:we,step_every_node:$})},k)}catch{}},xo=async $=>{if(!m.trim()){Be("工作流 YAML 不能为空");return}const W=b5(_.inputs,S);if(W.error){Be(W.error);return}let pe;const be=Mt.trim();if(be){const Oe=Number(be);if(!Number.isFinite(Oe)||Oe<=0){Be("单次运行预算上限 (cost_limit_usd) 必须为大于 0 的数字");return}pe=Oe}zi(!0),Be(null),Tt(null);try{const Oe=await Me("/v1/runs",t,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({tenant_id:k.trim()||"default",workflow:m,inputs:W.payload,model:A.trim()||void 0,cost_limit_usd:pe,run_mode:$,step_every_node:$==="debug"?Se:!1,breakpoints:$==="debug"?we:[]})},k);if(!Oe.ok){if(Oe.status===401)throw z(!0),V(!0),new Error("启动运行失败 (HTTP 401)：请在右上角弹出的「网关与凭据配置」中填写 Bearer Token");const Ve=await Oe.text().catch(()=>"");throw new Error(`启动运行失败 (HTTP ${Oe.status})${Ve?`: ${Ve.slice(0,180)}`:""}`)}z(!1);const dn=await tt(Dh,Oe,"POST /v1/runs (studio)");Pt([]),St([]),Nn([]),ti([]),ni({}),Ge(dn.run_id),Jt("loading"),ae($==="release"?"deliverables":"canvas")}catch(Oe){Be(Oe instanceof Error?Oe.message:String(Oe))}finally{zi(!1)}},Ss=async($,W)=>{if(me){Dn(!0),Be(null);try{const pe={action:$};W?.input_override&&(pe.input_override=W.input_override),W?.output_override&&(pe.output_override=W.output_override);const be=await Me(`/v1/runs/${encodeURIComponent(me)}:debug`,t,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify(pe)},k);if(!be.ok)throw new Error(`调试指令失败 (HTTP ${be.status})`);const Oe=await tt(qv,be,`POST /v1/runs/${me}:debug`);Tt(Oe.snapshot?.paused_at??null)}catch(pe){Be(pe instanceof Error?pe.message:String(pe))}finally{Dn(!1)}}},si=async()=>{if(me){Dn(!0);try{const $=await Me(`/v1/runs/${encodeURIComponent(me)}:cancel`,t,{method:"POST"},k);if($.ok){const W=await tt(hw,$,`POST /v1/runs/${me}:cancel`);rt(W.run),Tt(null),Jt("closed"),En.current?.abort()}}catch($){Be($ instanceof Error?$.message:String($))}finally{Dn(!1)}}},$n=async()=>{if(me){le(!0),Re(!0),wt(null),Ce(null);try{const $=await Me(`/v1/runs/${encodeURIComponent(me)}/resume-plan`,t,{},k);if(!$.ok){const pe=await $.text().catch(()=>"");throw new Error($.status===404?"当前运行尚未落盘可用的断点记录（例如首个节点在完成前即失败）":`读取续跑计划失败 (HTTP ${$.status})${pe?`: ${pe.slice(0,140)}`:""}`)}const W=await tt(GA,$,`GET /v1/runs/${me}/resume-plan`);Ce(W)}catch($){wt($ instanceof Error?$.message:String($))}finally{Re(!1)}}},wn=async()=>{if(!(!me||Vn)){Ws(!0),Be(null);try{const $=await Me(`/v1/runs/${encodeURIComponent(me)}:resume`,t,{method:"POST"},k);if(!$.ok){const pe=await $.text().catch(()=>"");throw new Error(`断点续跑启动失败 (HTTP ${$.status})${pe?`: ${pe.slice(0,160)}`:""}`)}const W=await tt(Ml,$,`POST /v1/runs/${me}:resume`);rt(dr(W)),le(!1),I(`已从断点 #${Ne?.last_completed_node??"latest"} 恢复运行`),Jt("loading")}catch($){wt($ instanceof Error?$.message:String($))}finally{Ws(!1)}}},Ri=async($,W,pe="once")=>{if(!(!me||Hs)){ei($);try{const be=await Me(`/v1/runs/${encodeURIComponent(me)}/approvals/${encodeURIComponent($)}`,t,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({decision:W,scope:pe,decided_by:"user:studio"})},k);if(!be.ok){const Oe=await be.text().catch(()=>"");throw new Error(`提交审批决策失败 (HTTP ${be.status})${Oe?`: ${Oe.slice(0,120)}`:""}`)}await tt(FA,be,`POST /v1/runs/${me}/approvals/${$}`),Nn(Oe=>Oe.filter(dn=>dn.id!==$))}catch(be){Be(be instanceof Error?be.message:String(be))}finally{ei(null)}}},Na=async($,W)=>{if(!me||wa)return;const pe=(W??Di[$]??"").trim();if(!pe){Be("请先填写你的回答后再提交");return}Sa($);try{const be=await Me(`/v1/runs/${encodeURIComponent(me)}/questions/${encodeURIComponent($)}`,t,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({answer:pe})},k);if(!be.ok){const Oe=await be.text().catch(()=>"");throw new Error(`提交问答回复失败 (HTTP ${be.status})${Oe?`: ${Oe.slice(0,120)}`:""}`)}await tt(WA,be,`POST /v1/runs/${me}/questions/${$}`),ti(Oe=>Oe.filter(dn=>dn.question_id!==$)),ni(Oe=>{const dn={...Oe};return delete dn[$],dn})}catch(be){Be(be instanceof Error?be.message:String(be))}finally{Sa(null)}},Ea=($,W)=>{const pe=W.target.files?.[0];if(!pe)return;if(pe.size>24*1024*1024){Be(`文件「${pe.name}」超过 24MB 浏览器内联上限，请改传公网 URL`),W.target.value="";return}const be=new FileReader;be.onload=()=>{typeof be.result=="string"&&(w(Oe=>({...Oe,[$]:be.result})),I(`已载入本地文件「${pe.name}」（${(pe.size/1024).toFixed(1)} KB）`))},be.readAsDataURL(pe),W.target.value=""},Ta=$=>{const W=$.target.files?.[0];if(!W)return;const pe=new FileReader;pe.onload=()=>{typeof pe.result=="string"&&(f("custom"),p(pe.result),w({}),I(`已加载本地工作流文件：${W.name}`))},pe.readAsText(W),$.target.value=""},Li=()=>{const $=l5(_.name),W=new Blob([m],{type:"text/yaml;charset=utf-8"}),pe=URL.createObjectURL(W),be=document.createElement("a");be.href=pe,be.download=$,document.body.appendChild(be),be.click(),be.remove(),window.setTimeout(()=>URL.revokeObjectURL(pe),1e3)},wo=async()=>{Be(null);try{const $=c.startsWith("personal:")?c.slice(9):void 0,{nextTemplates:W,preferencePatch:pe}=d5(O,{id:$,title:oe.trim()||_.name||"我的自定义工作流",yaml:m});R(W),vt(Eu,JSON.stringify(W));const be=W[0]?.id;be&&f(`personal:${be}`),Z(!1),Q("");const Oe=await Me("/v1/preferences",t,{method:"PUT",headers:{"Content-Type":"application/json"},body:JSON.stringify({preferences:pe})},k);Oe.ok?I(`已保存个人模板「${W[0]?.title??""}」并同步至偏好存储`):I(`已将模板缓存至本地浏览器（远端偏好同步返回 HTTP ${Oe.status}）`)}catch($){Be($ instanceof Error?$.message:String($))}},Sn=async($,W)=>{W.stopPropagation();const{nextTemplates:pe,preferencePatch:be}=f5(O,$);R(pe),vt(Eu,JSON.stringify(pe)),c===`personal:${$}`&&f("custom");try{await Me("/v1/preferences",t,{method:"PUT",headers:{"Content-Type":"application/json"},body:JSON.stringify({preferences:be})},k)}catch{}},ii=async()=>{if(!(!me||ee)){X(!0),Be(null);try{const $=await Me(`/v1/runs/${encodeURIComponent(me)}/artifacts.zip`,t,{},k);if(!$.ok){const Oe=await $.text().catch(()=>"");throw new Error(`批量导出产物失败 (HTTP ${$.status})${Oe?`: ${Oe.slice(0,140)}`:""}`)}const W=await $.blob(),pe=URL.createObjectURL(W),be=document.createElement("a");be.href=pe,be.download=`yanshi-run-${me.slice(0,8)}-artifacts.zip`,document.body.appendChild(be),be.click(),be.remove(),window.setTimeout(()=>URL.revokeObjectURL(pe),1500)}catch($){Be($ instanceof Error?$.message:String($))}finally{X(!1)}}},qi=($,W)=>{const pe=pz(m,$,{afterNodeId:W});if(!pe.ok){Be(pe.error);return}Be(null),f("custom"),p(pe.yaml),pe.createdNodeId&&ke(pe.createdNodeId)},Ca=($,W)=>{const pe=mz(m,$,W);if(!pe.ok){Be(pe.error);return}Be(null),f("custom"),p(pe.yaml),ke(W)},Is=($,W)=>{const pe=hz(m,$,W);if(!pe.ok){Be(pe.error);return}Be(null),f("custom"),p(pe.yaml)},Ui=$=>{const W=gz(m,$);if(!W.ok){Be(W.error);return}Be(null),f("custom"),p(W.yaml),ve(pe=>pe.filter(be=>be!==$)),ke(pe=>pe===$?null:pe)},pc=($,W)=>{const pe=yz(m,$,W);if(!pe.ok)return pe.error;if(Be(null),f("custom"),p(pe.yaml),pe.renamedTo){const be=pe.renamedTo;ke(be),ve(Oe=>Oe.map(dn=>dn===$?be:dn))}return null},ai=ww(),mc=D.trim()?`远端网关 · ${D.trim().replace(/^https?:\/\//,"")}`:typeof window<"u"&&window.__YANSHI_API_BASE__?"桌面内置引擎":Yv()?"移动端 · 待配网关":"Web 同源网关",hc=te?`${mc} · 待鉴权`:mc,Lr=$=>_.inputs.length===0?d.jsx("p",{className:"studio-muted-note",children:"该工作流未声明外部 inputs 参数，可直接启动运行。"}):d.jsx("div",{className:`studio-inputs-grid ${$?"studio-inputs-grid--compact":""}`,children:_.inputs.map(W=>{const pe=S[W.name]??Wu(W),be=W.kind==="string"&&(W.name.includes("source")||W.name.includes("notes")||W.name.includes("prompt")||pe.length>72);if(W.kind==="boolean"){const Ve=pe==="true";return d.jsxs("label",{className:"studio-bool-field",children:[d.jsx("input",{type:"checkbox",checked:Ve,onChange:yt=>w(on=>({...on,[W.name]:yt.target.checked?"true":"false"}))}),d.jsxs("div",{children:[d.jsx("strong",{children:W.name}),W.description&&d.jsx("small",{children:W.description})]})]},W.name)}const Oe=W.kind==="string"&&(W.name==="source"||W.name.endsWith("_source")||W.name.includes("pdf")||W.name.includes("video")||W.name.includes("audio")),dn=pe.startsWith("data:");return d.jsxs("div",{className:"studio-input-field",children:[d.jsxs("div",{className:"studio-input-field__label",children:[d.jsxs("strong",{children:[W.name,W.required?d.jsx("em",{className:"studio-req",children:"*"}):null]}),d.jsxs("span",{className:"studio-input-field__meta",children:[Oe&&d.jsxs("label",{className:"studio-upload-btn",title:"选择本地 PDF / 音频 / 视频文件并转为内联 Data URI",children:["📎 选择文件",d.jsx("input",{type:"file",accept:".pdf,.mp4,.mkv,.webm,.mov,.mp3,.wav,.m4a,.srt,.vtt",onChange:Ve=>Ea(W.name,Ve)})]}),d.jsx("span",{children:W.kind})]})]}),W.description&&d.jsx("small",{className:"studio-input-field__desc",children:W.description}),dn?d.jsxs("div",{className:"studio-datauri-pill",children:[d.jsxs("span",{children:["已载入内联文件数据（",Math.round(pe.length/1024)," KB Data URI）"]}),d.jsx("button",{type:"button",className:"studio-btn-ghost studio-btn-sm",onClick:()=>w(Ve=>({...Ve,[W.name]:Wu(W)})),children:"清除并改填 URL"})]}):W.kind==="json"||be?d.jsx("textarea",{"aria-label":W.name,rows:W.kind==="json"?3:2,value:pe,onChange:Ve=>w(yt=>({...yt,[W.name]:Ve.target.value}))}):d.jsx("input",{"aria-label":W.name,type:W.kind==="number"?"number":"text",value:pe,onChange:Ve=>w(yt=>({...yt,[W.name]:Ve.target.value}))})]},W.name)})}),Aa=_.nodes.length>0?Math.min(100,Math.round(Tn/_.nodes.length*100)):0;return d.jsxs("div",{className:`studio-shell studio-shell--${o}`,"data-mobile-pane":P,children:[d.jsxs("header",{className:"studio-topbar",children:[d.jsxs("div",{className:"studio-brand",children:[d.jsx("span",{className:"studio-brand__mark","aria-hidden":"true",children:"◈"}),d.jsxs("div",{children:[d.jsxs("div",{className:"studio-brand__row",children:[d.jsx("h1",{children:"Yanshi Studio"}),d.jsxs("span",{className:"studio-workflow-pill",children:[_.name||"未命名工作流"," · ",_.nodes.length," 节点"]})]}),d.jsx("p",{className:"studio-brand__sub",children:"可视化编排 · 单步 I/O 调试与一键发布运行工作台"})]})]}),d.jsxs("div",{className:"studio-mode-switch",role:"group","aria-label":"运行姿态切换（调试状态 / 发布状态）",children:[d.jsxs("button",{type:"button","data-testid":"mode-switch-debug",className:`studio-mode-btn ${o==="debug"?"studio-mode-btn--active":""}`,"aria-pressed":o==="debug",onClick:()=>rs("debug"),children:["🔍 调试状态",d.jsx("small",{children:"每步暂停观测 I/O"})]}),d.jsxs("button",{type:"button","data-testid":"mode-switch-release",className:`studio-mode-btn ${o==="release"?"studio-mode-btn--active":""}`,"aria-pressed":o==="release",onClick:()=>rs("release"),children:["🚀 发布状态",d.jsx("small",{children:"一键全速运行"})]})]}),d.jsxs("div",{className:"studio-topbar__right",children:[d.jsxs("div",{className:"studio-conn-anchor",children:[d.jsxs("button",{type:"button",className:"studio-btn-ghost studio-btn-sm","aria-expanded":B,onClick:()=>V($=>!$),children:[d.jsx("span",{className:`studio-conn-dot ${!te&&(ai||!Yv())?"on":"warn"}`}),hc]}),B&&d.jsxs("div",{className:"studio-conn-popover",role:"dialog","aria-label":"网关与凭据配置",children:[d.jsxs("label",{children:[d.jsx("span",{children:"网关地址 API Base URL（留空使用当前引擎）"}),d.jsx("input",{type:"url",placeholder:"例如 http://192.168.1.10:8080",value:D,onChange:$=>{U($.target.value),uO($.target.value)}})]}),d.jsxs("label",{children:[d.jsx("span",{children:"租户 Tenant ID"}),d.jsx("input",{type:"text",value:k,onChange:$=>j($.target.value)})]}),d.jsxs("label",{children:[d.jsx("span",{children:"Bearer Token（仅保存在当前会话）"}),d.jsx("input",{type:"password",autoComplete:"off",placeholder:"可选鉴权令牌",value:t,onChange:$=>s($.target.value)})]}),d.jsxs("div",{className:"studio-conn-popover__foot",children:[d.jsxs("small",{children:["当前生效：",ai||"同源 /ui 网关","（校验源：",b==="server"?"Rust 引擎":"本地解析","）"]}),d.jsx("button",{type:"button",className:"studio-btn-primary studio-btn-sm",onClick:()=>V(!1),children:"完成"})]})]})]}),d.jsxs("nav",{className:"studio-aux-links","aria-label":"辅助控制台导航",children:[d.jsx(ln,{to:"/ui/notebooks","data-testid":"studio-nav-notebooks",children:"📚 内容笔记本"}),(me||nt.length>0)&&d.jsx(ln,{to:eo(me||nt[0].id,nt.find($=>$.id!==(me||nt[0].id))?.id),children:"运行对比"}),d.jsx(ln,{to:"/ui/console",children:"经典事件台"}),d.jsx(ln,{to:"/ui/harness/sessions",children:"Harness"}),d.jsx(ln,{to:"/ui/diagnostics",children:"诊断"})]})]})]}),te&&d.jsxs("div",{className:"studio-alert studio-alert--warn",role:"alert","data-testid":"studio-auth-required-alert",children:[d.jsxs("span",{children:["🔐 当前网关已启用访问鉴权（HTTP 401）：请填入服务端 ",d.jsx("code",{children:"YANSHI_API_TOKEN"})," ","以开启 Rust 引擎静态校验、运行历史与在线执行。"]}),d.jsx("button",{type:"button",onClick:()=>V(!0),"data-testid":"studio-open-token-popover",children:"填写 Bearer Token"})]}),Qt&&d.jsxs("div",{className:"studio-alert studio-alert--error",role:"alert",children:[d.jsx("span",{children:Qt}),d.jsx("button",{type:"button",onClick:()=>Be(null),children:"关闭"})]}),L&&d.jsxs("div",{className:"studio-alert studio-alert--info",role:"status",children:[d.jsx("span",{children:L}),d.jsx("button",{type:"button",onClick:()=>I(null),children:"关闭"})]}),!_.diagnostics.valid&&_.diagnostics.issues.length>0&&d.jsx("div",{className:"studio-alert studio-alert--warn",role:"alert",children:d.jsxs("span",{children:["工作流校验问题：",_.diagnostics.issues.join("；")]})}),Je.costLimitExceeded&&d.jsx("div",{className:"studio-alert studio-alert--error",role:"alert","data-testid":"cost-limit-alert",children:d.jsxs("span",{children:["💰 预算熔断已触发：在节点 #",Je.costLimitExceeded.nodeId," 处累计花费"," ",Xa(Je.costLimitExceeded.spentUsd)," 已超过预算上限"," ",Xa(Je.costLimitExceeded.limitUsd)]})}),vs.length>0&&d.jsx("section",{className:"studio-run-approvals-banner",role:"region","aria-label":"运行期工具调用审批","data-testid":"run-approvals-banner",children:vs.map($=>{const W=Hs===$.id;return d.jsxs("div",{className:"studio-run-approval-card",children:[d.jsxs("div",{className:"studio-run-approval-card__info",children:[d.jsxs("div",{className:"studio-run-approval-card__top",children:[d.jsxs("span",{className:"studio-approval-risk","data-risk":$.risk,children:["审批拦截 · ",$.risk.toUpperCase()]}),d.jsxs("strong",{children:["工具：",$.tool]}),d.jsxs("small",{children:["Step #",$.step_index]})]}),d.jsx("p",{children:$.reason}),$.params_summary!==void 0&&$.params_summary!==null&&d.jsx("pre",{className:"studio-run-approval-card__params",children:typeof $.params_summary=="string"?$.params_summary:JSON.stringify($.params_summary,null,2)})]}),d.jsxs("div",{className:"studio-run-approval-card__actions",children:[d.jsx("button",{type:"button",className:"studio-btn-primary studio-btn-sm",disabled:W,onClick:()=>{Ri($.id,"allow","once")},children:"✓ 允许一次"}),d.jsx("button",{type:"button",className:"studio-btn-secondary studio-btn-sm",disabled:W,onClick:()=>{Ri($.id,"allow","session")},children:"✓ 本轮始终允许"}),d.jsx("button",{type:"button",className:"studio-btn-danger-ghost studio-btn-sm",disabled:W,onClick:()=>{Ri($.id,"deny_and_stop","once")},children:"✕ 拒绝并中止"})]})]},$.id)})}),xa.length>0&&d.jsx("section",{className:"studio-run-questions-banner",role:"region","aria-label":"架构规划多轮澄清问答","data-testid":"run-questions-banner",children:xa.map($=>{const W=wa===$.question_id,pe=Di[$.question_id]??"";return d.jsxs("div",{className:"studio-run-question-card",children:[d.jsxs("div",{className:"studio-run-question-card__header",children:[d.jsx("span",{className:"studio-question-badge",children:"💬 Agent 需求澄清"}),d.jsxs("strong",{children:["节点：",$.node_id]}),d.jsx("small",{children:"回答后工作流将自动继续执行"})]}),d.jsx("div",{className:"studio-run-question-card__prompt",children:$.question}),d.jsxs("div",{className:"studio-run-question-card__reply",children:[d.jsx("textarea",{rows:3,"aria-label":`回答节点 ${$.node_id} 的提问`,placeholder:"输入你的业务约束、技术栈偏好或规模预期（Ctrl/Cmd + Enter 提交）…",value:pe,disabled:W,onChange:be=>ni(Oe=>({...Oe,[$.question_id]:be.target.value})),onKeyDown:be=>{(be.metaKey||be.ctrlKey)&&be.key==="Enter"&&(be.preventDefault(),Na($.question_id))}}),d.jsxs("div",{className:"studio-run-question-card__actions",children:[d.jsx("button",{type:"button",className:"studio-btn-primary studio-btn-sm",disabled:W||!pe.trim(),onClick:()=>{Na($.question_id)},children:W?"提交中…":"发送回答并继续 →"}),d.jsx("button",{type:"button",className:"studio-btn-secondary studio-btn-sm",disabled:W,onClick:()=>{Na($.question_id,"其余细节请直接采用生产级主流工程最佳实践默认方案，无需继续追问，直接输出完整规格方案。")},children:"按工程最佳实践默认继续"})]})]})]},$.question_id)})}),o==="debug"?d.jsxs("main",{className:`studio-debug-grid ${Y?"studio-debug-grid--collapsed":""}`,children:[d.jsxs("aside",{className:"studio-pane studio-pane--config","aria-label":"工作流预设与输入配置",children:[d.jsxs("div",{className:"studio-pane__head",children:[d.jsx("strong",{children:"工作流与输入配置"}),d.jsx("button",{type:"button",className:"studio-btn-ghost studio-btn-sm studio-desktop-only",onClick:()=>ce($=>!$),children:Y?"展开":"收起"})]}),!Y&&d.jsxs("div",{className:"studio-pane__scroll",children:[d.jsxs("div",{className:"studio-section",children:[d.jsxs("div",{className:"studio-section__head",children:[d.jsxs("span",{children:["工作流模板库 (",ws.length,")"]}),d.jsxs("div",{className:"studio-file-actions",children:[d.jsxs("label",{className:"studio-upload-btn",title:"从磁盘导入本地 .yml/.yaml 工作流",children:["📂 打开 .yml",d.jsx("input",{type:"file",accept:".yml,.yaml",onChange:Ta})]}),d.jsx("button",{type:"button",className:"studio-upload-btn","data-testid":"export-yaml-btn",title:"将当前工作流 YAML 保存到本地磁盘",onClick:Li,children:"💾 导出 .yml"}),d.jsx("button",{type:"button",className:"studio-upload-btn","data-testid":"bookmark-template-btn",title:"收藏当前工作流到个人模板库",onClick:()=>{Q(_.name||"我的自定义工作流"),Z($=>!$)},children:"★ 收藏"})]})]}),q&&d.jsxs("div",{className:"studio-save-tpl-box",children:[d.jsx("input",{type:"text","aria-label":"个人模板名称",placeholder:"输入收藏模板名称…",value:oe,onChange:$=>Q($.target.value)}),d.jsx("button",{type:"button",className:"studio-btn-primary studio-btn-sm",onClick:()=>{wo()},children:"确认保存"}),d.jsx("button",{type:"button",className:"studio-btn-ghost studio-btn-sm",onClick:()=>Z(!1),children:"取消"})]}),d.jsx("div",{className:"studio-category-pills",role:"tablist","aria-label":"模板分类筛选",children:["全部","课件与视频","研究报告与信息图","导图与工程图","播客音频","互动学习","免 Key 演练","个人收藏"].map($=>d.jsx("button",{type:"button",role:"tab","aria-selected":T===$,className:`studio-cat-pill ${T===$?"studio-cat-pill--active":""}`,onClick:()=>E($),children:$},$))}),d.jsx("div",{className:"studio-preset-list",children:os.length===0?d.jsx("p",{className:"studio-muted-note",children:"当前分类暂无模板（点击上方「★ 收藏」可将任意工作流存入个人收藏）。"}):os.map($=>d.jsxs("div",{className:`studio-preset-card ${c===$.id?"studio-preset-card--active":""}`,children:[d.jsxs("button",{type:"button",className:"studio-preset-card__main",onClick:()=>vo($.id),children:[d.jsxs("div",{className:"studio-preset-card__top",children:[d.jsx("strong",{children:$.title}),d.jsx("span",{children:$.badge})]}),d.jsx("small",{children:$.subtitle})]}),$.personalId&&d.jsx("button",{type:"button",className:"studio-preset-card__delete","aria-label":`删除个人模板 ${$.title}`,title:"移除该收藏模板",onClick:W=>{Sn($.personalId,W)},children:"✕"})]},$.id))})]}),d.jsxs("div",{className:"studio-section",children:[d.jsx("div",{className:"studio-section__head",children:d.jsxs("span",{children:["调试输入参数 (",_.inputs.length,")"]})}),d.jsxs("label",{className:"studio-input-field studio-model-override-field",children:[d.jsxs("div",{className:"studio-input-field__label",children:[d.jsx("strong",{children:"全局推理模型覆盖 (model)"}),d.jsx("span",{children:"optional"})]}),d.jsx("small",{className:"studio-input-field__desc",children:"为未显式指定模型的 LLM / Agent 节点统一注入模型名（如官方生成器模板）"}),d.jsx("input",{type:"text",placeholder:"例如 gemini-2.5-flash / gpt-4o-mini（留空按节点配置）",value:A,onChange:$=>N($.target.value)})]}),d.jsxs("label",{className:"studio-input-field studio-cost-limit-field",children:[d.jsxs("div",{className:"studio-input-field__label",children:[d.jsx("strong",{children:"单次运行预算上限 USD (cost_limit_usd)"}),d.jsx("span",{children:"optional"})]}),d.jsxs("small",{className:"studio-input-field__desc",children:["超出自动熔断防超支",typeof _.cost_limit_usd=="number"?`（YAML 已声明上限 $${_.cost_limit_usd}，取更严格值）`:""]}),d.jsx("input",{type:"number",min:"0.0001",step:"0.01","data-testid":"studio-cost-limit-input",placeholder:"例如 1.50（留空不设额外上限）",value:Mt,onChange:$=>Bs($.target.value)})]}),Lr(!0)]}),d.jsxs("div",{className:"studio-section",children:[d.jsxs("div",{className:"studio-section__head",children:[d.jsx("span",{children:"YAML 编排源码"}),d.jsx("button",{type:"button",className:"studio-link-btn",onClick:()=>_e($=>!$),children:de?"收起编辑器":"展开编辑 YAML"})]}),de&&d.jsx(rd,{testId:"studio-yaml-editor",ariaLabel:"YAML 编排源码",value:m,minHeight:280,onExpandToggle:()=>G(!0),onChange:$=>{f("custom"),p($)}})]}),nt.length>0&&d.jsxs("div",{className:"studio-section",children:[d.jsx("div",{className:"studio-section__head",children:d.jsx("span",{children:"最近运行记录"})}),d.jsx("ul",{className:"studio-recent-runs",children:nt.slice(0,6).map($=>d.jsxs("li",{className:"studio-recent-runs__item",children:[d.jsxs("button",{type:"button",className:$.id===me?"active":"",onClick:()=>{Ge($.id),Jt("loading")},children:[d.jsx("span",{children:$.id.slice(0,8)}),d.jsx("em",{children:$.status}),d.jsx("small",{children:Zn($.started_at)})]}),d.jsx(ln,{className:"studio-compare-pill",title:me&&$.id!==me?`将当前运行与 ${$.id.slice(0,8)} 对比`:"以此运行为基准开启对比",to:me&&$.id!==me?eo(me,$.id):eo($.id,nt.find(W=>W.id!==$.id)?.id),children:"对比"})]},$.id))})]})]})]}),d.jsxs("section",{className:"studio-pane studio-pane--canvas","aria-label":"可视化 DAG 调试画布",children:[d.jsxs("div",{className:"studio-debug-deck",role:"toolbar","aria-label":"单步调试传输控台",children:[d.jsxs("div",{className:"studio-debug-deck__primary",children:[Y&&d.jsx("button",{type:"button",className:"studio-btn-ghost studio-btn-sm studio-desktop-only",onClick:()=>ce(!1),children:"展开左栏"}),d.jsx("button",{type:"button","data-testid":"launch-debug-run",className:"studio-btn-primary",disabled:zn||!_.diagnostics.valid,onClick:()=>{xo("debug")},children:zn?"启动中…":"🔍 启动单步调试"}),d.jsxs("label",{className:"studio-step-toggle",title:"开启后每完成一个节点自动挂起，等待您观测完输入与输出后再进入下一步",children:[d.jsx("input",{type:"checkbox",checked:Se,onChange:$=>{ja($.target.checked)}}),d.jsx("span",{children:"每步暂停观测"})]}),d.jsx("div",{className:"studio-deck-divider"}),d.jsx("button",{type:"button","data-testid":"debug-step-next",className:"studio-btn-step",disabled:!hn||Us,onClick:()=>{Ss("step")},children:"▶ 单步下一步"}),d.jsx("button",{type:"button","data-testid":"debug-continue-run",className:"studio-btn-secondary",disabled:!hn||Us,onClick:()=>{Ss("continue")},children:"⏭ 继续跑完"}),d.jsx("button",{type:"button",className:"studio-btn-danger-ghost",disabled:!zt||Us,onClick:()=>{si()},children:"⏹ 终止"}),Dt&&d.jsx("button",{type:"button","data-testid":"open-resume-plan-btn",className:"studio-btn-secondary studio-btn-sm",onClick:()=>{$n()},children:"↻ 断点续跑"}),me&&(lt.length>0||yn.length>0)&&d.jsx("button",{type:"button","data-testid":"download-artifacts-zip-debug",className:"studio-btn-ghost studio-btn-sm",disabled:ee,onClick:()=>{ii()},children:ee?"打包中…":"📦 打包导出 (.zip)"})]}),d.jsxs("div",{className:"studio-debug-deck__status",children:[(Je.totalCostUsd>0||Cn!==null)&&d.jsxs("span",{className:`studio-spend-pill ${Je.costLimitExceeded?"studio-spend-pill--exceeded":""}`,"data-testid":"studio-live-spend-pill",children:["💰 ",Xa(Je.totalCostUsd),Cn!==null?` / ${Xa(Cn)}`:""]}),hn?d.jsxs("span",{className:"studio-live-pill studio-live-pill--paused",children:["⏸ 已暂停于 #",hn.node_id,"（",hn.phase==="before_node"?"执行前断点":"本步已完成","）"]}):zt?d.jsxs("span",{className:"studio-live-pill studio-live-pill--running",children:["● 运行中 · ",Tn,"/",_.nodes.length," 步"]}):Ke?d.jsxs("span",{className:"studio-live-pill studio-live-pill--done",children:["状态：",Ke.status," · ",Tn,"/",_.nodes.length," 步",typeof Je.workflowDurationMs=="number"?` (${(Je.workflowDurationMs/1e3).toFixed(2)}s)`:""]}):d.jsxs("span",{className:"studio-live-pill",children:["就绪 · 共 ",_.levels.length," 层拓扑阶段"]})]})]}),d.jsx(Z6,{nodes:_.nodes,edges:_.edges,levels:_.levels,executionOrder:_.execution_order,telemetryByNode:Je.telemetryByNode,breakpoints:we,selectedNodeId:xe,editable:!zt,onSelectNode:$=>{ke($),window.innerWidth<860&&ae("inspector")},onToggleBreakpoint:$=>{ka($)},onAddNode:qi,onConnectNodes:Ca,onDisconnectEdge:Is,onDeleteNode:Ui})]}),d.jsx("aside",{className:"studio-pane studio-pane--inspector",children:d.jsx(xz,{runId:me,apiToken:t,tenantId:k,node:gn,allNodes:_.nodes,editable:!zt,telemetry:gn?Je.telemetryByNode[gn.id]:void 0,capturedInput:gn?Je.nodeInputs[gn.id]:void 0,capturedOutput:gn?Je.nodeOutputs[gn.id]:void 0,pausedAt:hn,hasBreakpoint:gn?we.includes(gn.id):!1,onToggleBreakpoint:$=>{ka($)},onStepDebug:$=>{Ss("step",$)},onContinueDebug:$=>{Ss("continue",$)},debugActionBusy:Us,onApplyNodePatch:pc,onDeleteNode:_.nodes.length>1?Ui:void 0})}),d.jsxs("section",{className:"studio-pane studio-pane--mobile-deliverables",children:[d.jsxs("div",{className:"studio-pane__head",children:[d.jsxs("strong",{children:["本轮产出预览 (",yn.length,")"]}),me&&(lt.length>0||yn.length>0)&&d.jsx("button",{type:"button",className:"studio-btn-ghost studio-btn-sm",disabled:ee,onClick:()=>{ii()},children:ee?"打包中…":"📦 导出 .zip"})]}),d.jsx("div",{className:"studio-pane__scroll",children:yn.length===0&&!un?d.jsx("p",{className:"studio-muted-note",children:"运行产生文件或文本结果后将在此汇总展示。"}):d.jsxs("div",{className:"studio-media-list",children:[yn.map($=>d.jsx(ql,{item:$,apiToken:t,tenantId:k},$.key)),un&&d.jsxs("article",{className:"studio-text-deliverable",children:[d.jsx("header",{children:d.jsxs("strong",{children:["文本汇总产出（#",un.nodeId,"）"]})}),d.jsx("pre",{children:un.text})]})]})})]})]}):d.jsxs("main",{className:"studio-release-grid",children:[d.jsxs("section",{className:"studio-release-form-card","aria-label":"应用运行参数配置",children:[d.jsxs("div",{className:"studio-release-card__head",children:[d.jsxs("div",{children:[d.jsx("span",{className:"studio-kicker",children:"RELEASE APP · 一键全速运行"}),d.jsx("h2",{children:_.name||"工作流应用"})]}),d.jsxs("select",{"aria-label":"选择应用工作流",className:"studio-preset-select",value:c,onChange:$=>vo($.target.value),children:[ws.map($=>d.jsx("option",{value:$.id,children:$.category?`[${$.category}] ${$.title}`:$.title},$.id)),c==="custom"&&d.jsx("option",{value:"custom",children:"自定义导入 YAML"})]})]}),d.jsxs("div",{className:"studio-release-form-body",children:[d.jsxs("label",{className:"studio-input-field studio-model-override-field",children:[d.jsxs("div",{className:"studio-input-field__label",children:[d.jsx("strong",{children:"全局推理模型覆盖 (model)"}),d.jsx("span",{children:"optional"})]}),d.jsx("small",{className:"studio-input-field__desc",children:"为未显式指定模型的 LLM / Agent 节点统一注入模型名"}),d.jsx("input",{type:"text",placeholder:"例如 gemini-2.5-flash / gpt-4o-mini（留空按节点配置）",value:A,onChange:$=>N($.target.value)})]}),d.jsxs("label",{className:"studio-input-field studio-cost-limit-field",children:[d.jsxs("div",{className:"studio-input-field__label",children:[d.jsx("strong",{children:"单次运行预算上限 USD (cost_limit_usd)"}),d.jsx("span",{children:"optional"})]}),d.jsxs("small",{className:"studio-input-field__desc",children:["超出自动熔断防超支",typeof _.cost_limit_usd=="number"?`（YAML 已声明上限 $${_.cost_limit_usd}）`:""]}),d.jsx("input",{type:"number",min:"0.0001",step:"0.01","data-testid":"studio-cost-limit-input",placeholder:"例如 1.50（留空不设额外上限）",value:Mt,onChange:$=>Bs($.target.value)})]}),Lr(!1)]}),d.jsxs("div",{className:"studio-release-cta-bar",children:[d.jsx("button",{type:"button","data-testid":"launch-release-run",className:"studio-btn-launch",disabled:zn||!_.diagnostics.valid,onClick:()=>{xo("release")},children:zn?"正在提交并发任务…":zt?"🚀 重新一键运行整个流程":"🚀 一键运行整个流程"}),Dt&&d.jsx("button",{type:"button","data-testid":"open-resume-plan-release-btn",className:"studio-btn-secondary",onClick:()=>{$n()},children:"↻ 从断点续跑"}),zt&&d.jsx("button",{type:"button",className:"studio-btn-danger-ghost",onClick:()=>{si()},children:"终止当前运行"})]})]}),d.jsxs("section",{className:"studio-release-stage-card","aria-label":"全流程进度与成果展台",children:[d.jsxs("div",{className:"studio-progress-panel",children:[d.jsxs("div",{className:"studio-progress-panel__top",children:[d.jsxs("div",{children:[d.jsx("strong",{children:zt?"正在全速并发执行流水线…":Ke?.status==="succeeded"||Tn>0&&Tn===_.nodes.length?"✓ 全流程执行完成":Je.workflowError||Ke?.status==="failed"?"✕ 执行过程中发生异常":"等待启动"}),d.jsxs("small",{children:["已完成 ",Tn," / ",_.nodes.length," 个阶段",typeof Je.workflowDurationMs=="number"?` · 总耗时 ${(Je.workflowDurationMs/1e3).toFixed(2)}s`:""]})]}),d.jsxs("div",{className:"studio-progress-panel__right",children:[(Je.totalCostUsd>0||Cn!==null)&&d.jsxs("span",{className:`studio-spend-pill ${Je.costLimitExceeded?"studio-spend-pill--exceeded":""}`,children:["💰 ",Xa(Je.totalCostUsd),Cn!==null?` / ${Xa(Cn)}`:""]}),d.jsxs("span",{className:"studio-pct",children:[Aa,"%"]}),me&&d.jsx("button",{type:"button",className:"studio-link-btn",onClick:()=>rs("debug"),children:"切至调试视图核查节点 I/O →"})]})]}),d.jsx("div",{className:"studio-progress-bar",role:"progressbar","aria-valuemin":0,"aria-valuemax":100,"aria-valuenow":Aa,children:d.jsx("div",{className:"studio-progress-bar__fill",style:{width:`${Aa}%`}})}),d.jsx("ol",{className:"studio-stage-pills","aria-label":"阶段进度列表",children:_.execution_order.map(($,W)=>{const pe=Je.telemetryByNode[$]?.status??"idle";return d.jsxs("li",{className:`studio-stage-pill studio-stage-pill--${pe}`,children:[d.jsx("span",{children:W+1}),d.jsx("strong",{children:$})]},$)})})]}),Je.workflowError&&d.jsxs("div",{className:"studio-node-error",role:"alert",children:[d.jsx("strong",{children:"执行失败详情："})," ",Je.workflowError]}),d.jsxs("div",{className:"studio-deliverables-showcase",children:[d.jsxs("div",{className:"studio-section__head",children:[d.jsxs("span",{children:["最终成果展台 (",yn.length+(un?1:0),")"]}),d.jsxs("div",{className:"studio-deliverables-head-right",children:[me&&(lt.length>0||yn.length>0)&&d.jsx("button",{type:"button","data-testid":"download-artifacts-zip-release",className:"studio-btn-secondary studio-btn-sm",disabled:ee,onClick:()=>{ii()},children:ee?"正在打包…":"📦 一键打包下载 (.zip)"}),me&&d.jsxs("small",{children:["Run ID: ",me.slice(0,12),"…"]})]})]}),yn.length===0&&!un?d.jsx("div",{className:"studio-showcase-placeholder",children:d.jsxs("p",{children:["点击左侧 ",d.jsx("strong",{children:"「🚀 一键运行整个流程」"}),"，系统将并发调度全部无依赖阶段，并在此自动呈现合成好的 1080p 讲解视频、交互式 HTML 幻灯片、PPTX/PDF 课件或文本讲义。"]})}):d.jsxs("div",{className:"studio-deliverables-grid",children:[yn.map($=>d.jsx(ql,{item:$,apiToken:t,tenantId:k},$.key)),un&&d.jsxs("article",{className:"studio-text-deliverable",children:[d.jsx("header",{children:d.jsxs("strong",{children:["最终讲义 / 文本输出（#",un.nodeId,"）"]})}),d.jsx("pre",{"data-testid":"release-final-text",children:un.text})]})]})]})]})]}),d.jsx("nav",{className:"studio-mobile-nav","aria-label":"移动端视图切换",children:o==="debug"?d.jsxs(d.Fragment,{children:[d.jsx("button",{type:"button",className:P==="config"?"active":"",onClick:()=>ae("config"),children:"配置"}),d.jsx("button",{type:"button",className:P==="canvas"?"active":"",onClick:()=>ae("canvas"),children:"拓扑调试"}),d.jsxs("button",{type:"button",className:P==="inspector"?"active":"",onClick:()=>ae("inspector"),children:["I/O 观测",hn?d.jsx("i",{className:"studio-nav-dot"}):null]}),d.jsxs("button",{type:"button",className:P==="deliverables"?"active":"",onClick:()=>ae("deliverables"),children:["成果 (",yn.length,")"]})]}):d.jsxs(d.Fragment,{children:[d.jsx("button",{type:"button",className:P==="config"?"active":"",onClick:()=>ae("config"),children:"参数配置"}),d.jsxs("button",{type:"button",className:P!=="config"?"active":"",onClick:()=>ae("deliverables"),children:["进度与成果 (",yn.length,")"]})]})}),F&&d.jsx("div",{className:"studio-yaml-modal-backdrop",role:"dialog","aria-modal":"true","aria-label":"放大编辑工作流 YAML",onKeyDown:$=>{$.key==="Escape"&&G(!1)},children:d.jsxs("div",{className:"studio-yaml-modal",children:[d.jsxs("header",{className:"studio-yaml-modal__head",children:[d.jsxs("div",{children:[d.jsx("strong",{children:"YAML 编排源码编辑器"}),d.jsx("span",{className:_.diagnostics.valid?"studio-yaml-modal__badge ok":"studio-yaml-modal__badge warn",children:_.diagnostics.valid?`✓ 拓扑有效 · ${_.nodes.length} 节点 / ${_.edges.length} 边`:`⚠ ${_.diagnostics.issues[0]??"语法或依赖有误"}`})]}),d.jsx("button",{type:"button",className:"studio-btn-primary studio-btn-sm",onClick:()=>G(!1),children:"完成并关闭 (Esc)"})]}),d.jsx(rd,{ariaLabel:"放大编辑工作流 YAML",value:m,isExpanded:!0,onExpandToggle:()=>G(!1),onChange:$=>{f("custom"),p($)}})]})}),va&&d.jsx("div",{className:"studio-yaml-modal-backdrop",role:"dialog","aria-modal":"true","aria-label":"断点续跑计划预览",onKeyDown:$=>{$.key==="Escape"&&le(!1)},children:d.jsxs("div",{className:"studio-resume-modal",children:[d.jsxs("header",{className:"studio-yaml-modal__head",children:[d.jsxs("div",{children:[d.jsx("strong",{children:"↻ 断点续跑计划预览 (Checkpoint Resume Plan)"}),d.jsxs("span",{className:"studio-yaml-modal__badge ok",children:["Run ",me.slice(0,12),"…"]})]}),d.jsx("button",{type:"button",className:"studio-btn-ghost studio-btn-sm",onClick:()=>le(!1),children:"关闭 (Esc)"})]}),d.jsx("div",{className:"studio-resume-modal__body",children:Te?d.jsx("p",{className:"studio-muted-note",children:"正在加载已落盘的 Checkpoint 状态与幂等性分析…"}):ct?d.jsx("div",{className:"studio-node-error",role:"alert",children:ct}):Ne?d.jsxs(d.Fragment,{children:[d.jsxs("div",{className:"studio-resume-summary-grid",children:[d.jsxs("div",{className:"studio-resume-stat",children:[d.jsx("span",{children:"最近已完成断点节点"}),d.jsxs("strong",{children:["#",Ne.last_completed_node]})]}),d.jsxs("div",{className:"studio-resume-stat",children:[d.jsx("span",{children:"断点保存时间"}),d.jsx("strong",{children:Zn(Ne.created_at)})]}),d.jsxs("div",{className:"studio-resume-stat",children:[d.jsx("span",{children:"未完成工具调用"}),d.jsxs("strong",{children:[Ne.summary.total," 项（跳过 ",Ne.summary.to_skip," · 重放"," ",Ne.summary.to_replay," · 需人工"," ",Ne.summary.requires_manual,"）"]})]})]}),d.jsxs("div",{className:"studio-resume-nodes-split",children:[d.jsxs("div",{className:"studio-resume-nodes-col",children:[d.jsx("h4",{children:"✓ 将跳过并直接复用缓存结果"}),d.jsx("div",{className:"studio-resume-tags",children:_.execution_order.filter($=>{const W=Je.telemetryByNode[$]?.status;return W==="succeeded"||W==="skipped"}).map($=>d.jsxs("span",{className:"studio-resume-tag studio-resume-tag--cached",children:["#",$]},$))})]}),d.jsxs("div",{className:"studio-resume-nodes-col",children:[d.jsx("h4",{children:"▶ 将从断点继续执行"}),d.jsx("div",{className:"studio-resume-tags",children:_.execution_order.filter($=>{const W=Je.telemetryByNode[$]?.status;return W!=="succeeded"&&W!=="skipped"}).map($=>d.jsxs("span",{className:"studio-resume-tag studio-resume-tag--rerun",children:["#",$]},$))})]})]}),Ne.tool_calls.length>0&&d.jsxs("div",{className:"studio-resume-tools",children:[d.jsx("h4",{children:"内嵌 Agent 未决工具调用恢复决策"}),d.jsx("ul",{children:Ne.tool_calls.map($=>d.jsxs("li",{children:[d.jsxs("strong",{children:["#",$.node_id," · ",$.tool]}),d.jsx("span",{"data-decision":$.decision,children:$.decision}),d.jsx("small",{children:$.reason})]},`${$.node_id}-${$.tool_call_id}`))})]}),Ne.summary.requires_manual>0&&d.jsx("div",{className:"studio-alert studio-alert--warn",role:"alert",children:d.jsxs("span",{children:["存在 ",Ne.summary.requires_manual," ","项非幂等外部副作用工具调用需要人工确认，自动续跑将被引擎安全拦截。"]})})]}):null}),d.jsxs("footer",{className:"studio-resume-modal__foot",children:[d.jsx("button",{type:"button",className:"studio-btn-ghost",onClick:()=>le(!1),children:"取消"}),d.jsx("button",{type:"button","data-testid":"confirm-resume-run-btn",className:"studio-btn-primary",disabled:Te||!Ne||Vn||Ne.summary.requires_manual>0,onClick:()=>{wn()},children:Vn?"正在恢复运行…":"▶ 确认从断点续跑"})]})]})})]})}const wh=[{kind:"report",title:"溯源研究报告",deliverableBadge:"MD · HTML · PDF",summary:"规划章节大纲并并行撰写正文，自动附带来源分块引用脚注。",fields:[{key:"focus",label:"报告聚焦主题",type:"text",defaultValue:"the main points, with their supporting evidence"},{key:"sections",label:"章节数量",type:"number",defaultValue:4,min:2,max:10},{key:"pdf",label:"同时排版 PDF（需本机安装 pandoc 与 typst）",type:"checkbox",defaultValue:!1}]},{kind:"mindmap",title:"结构化思维导图",deliverableBadge:"SVG · PNG · 交互 HTML",summary:"提炼多层级主题树，同时导出矢量图、高清位图与可折叠交互导图页。",fields:[{key:"focus",label:"导图梳理主线",type:"text",defaultValue:"the main ideas and how they relate"},{key:"depth",label:"展开层级深度 (2–4)",type:"number",defaultValue:3,min:2,max:5}]},{kind:"flashcards",title:"翻转记忆闪卡",deliverableBadge:"交互 HTML · JSON",summary:"按知识点生成一问一答记忆卡片，产出零依赖可点击翻转复习页。",fields:[{key:"focus",label:"闪卡考察重点",type:"text",defaultValue:"the key facts and ideas"},{key:"count",label:"卡片张数 (5–30)",type:"number",defaultValue:12,min:3,max:30}]},{kind:"quiz",title:"互动自测题库",deliverableBadge:"自测 HTML · JSON",summary:"生成带解析与出处引用的多项选择题，支持页面内即时判分与复盘。",fields:[{key:"focus",label:"测验出题范围",type:"text",defaultValue:"the key facts and ideas"},{key:"count",label:"题目数量 (3–20)",type:"number",defaultValue:8,min:3,max:20}]},{kind:"infographic",title:"核心指标信息长图",deliverableBadge:"SVG · PNG",summary:"浓缩关键数字指标、分栏要点与核心结论，渲染出版级视觉信息长图。",fields:[{key:"focus",label:"信息图突出重点",type:"text",defaultValue:"the key numbers and ideas"}]},{kind:"slides",title:"演示文稿课件",deliverableBadge:"PPTX · 矢量配图",summary:"生成带演讲备注与自动渲染 SVG 示意图的结构化演示幻灯片（需 pandoc）。",fields:[{key:"focus",label:"演示核心论点",type:"text",defaultValue:"the main ideas, for a general audience"},{key:"slides",label:"内容页数",type:"number",defaultValue:6,min:3,max:20},{key:"figure_languages",label:"配图语言",type:"text",defaultValue:"svg"}]},{kind:"audio_overview",title:"双人对话播客概览",deliverableBadge:"MP3 音频",summary:"编写主持人与嘉宾对谈台本，并行合成双声部语音并响度标准化拼接（需 TTS + ffmpeg）。",fields:[{key:"focus",label:"对谈讨论焦点",type:"text",defaultValue:"the main ideas, in plain language"},{key:"minutes",label:"预计时长（分钟）",type:"number",defaultValue:3,min:1,max:15},{key:"language",label:"对谈语言",type:"text",defaultValue:"Chinese"},{key:"tts_model",label:"TTS 语音模型名",type:"text",defaultValue:"tts-1"}]},{kind:"video_overview",title:"多镜头配音视频概览",deliverableBadge:"MP4 · SRT 字幕",summary:"编排分镜解说词，并行生成插画画面与旁白语音，合成软字幕 MP4 视频（需生图 + TTS + ffmpeg）。",fields:[{key:"focus",label:"视频讲解主线",type:"text",defaultValue:"the main ideas, in plain language"},{key:"scene_count",label:"分镜数量 (4–10)",type:"number",defaultValue:6,min:2,max:12},{key:"language",label:"旁白语言",type:"text",defaultValue:"Chinese"},{key:"image_model",label:"文生图模型名",type:"text",defaultValue:"gpt-image-2"},{key:"tts_model",label:"TTS 语音模型名",type:"text",defaultValue:"tts-1"}]}];function v5(e){const t=e.match(/^\/ui\/notebooks\/([^/]+)\/?$/);return t?decodeURIComponent(t[1]):null}function x5(e){const s=[];for(let a=0;a<e.length;a+=32768){const o=e.subarray(a,a+32768);s.push(String.fromCharCode(...o))}return btoa(s.join(""))}function w5(e,t){const s=(t??"").toLowerCase();if(s.startsWith("image/"))return"image";if(s.startsWith("audio/"))return"audio";if(s.startsWith("video/"))return"video";if(s.startsWith("text/html"))return"html";if(s.includes("subrip")||s.startsWith("text/vtt"))return"subtitle";if(s.includes("pdf")||s.includes("presentation")||s.includes("markdown"))return"document";const a=e.lastIndexOf("."),o=a>=0?e.slice(a+1).toLowerCase():"";return["png","jpg","jpeg","webp","gif","svg"].includes(o)?"image":["mp3","wav","ogg","m4a"].includes(o)?"audio":["mp4","webm"].includes(o)?"video":["html","htm"].includes(o)?"html":["srt","vtt"].includes(o)?"subtitle":"document"}function S5(e){const t=`/v1/runs/${encodeURIComponent(e.run_id)}/artifacts/${encodeURIComponent(e.id)}`;return{key:`${e.kind}:${e.id}`,nodeId:e.node_id,label:e.name,kind:w5(e.name,e.mime_type),apiPath:t,directUrl:tc(t),mimeType:e.mime_type??void 0,sizeBytes:e.size_bytes??void 0}}function k5(){const e={};for(const t of wh){const s={};for(const a of t.fields)s[a.key]=typeof a.defaultValue=="boolean"?a.defaultValue:String(a.defaultValue);e[t.kind]=s}return e}function j5(e){switch(e){case"ready":return"已就绪";case"ingesting":return"解析分块中…";case"pending":return"排队入库…";case"failed":return"入库失败";default:return e}}function Ax({initialNotebookId:e=null,apiToken:t,onTokenChange:s}){const[a,o]=C.useState(()=>bt(Ni,"default")),[l,c]=C.useState(()=>bt(ad,"")),[f,m]=C.useState([]),[p,_]=C.useState(e??""),[y,b]=C.useState(null),[v,S]=C.useState([]),[w,k]=C.useState([]),[j,A]=C.useState(""),[N,T]=C.useState(!1),[E,O]=C.useState("text"),[R,q]=C.useState(""),[Z,oe]=C.useState(""),[Q,ee]=C.useState(""),[X,L]=C.useState(null),[I,D]=C.useState(!1),[U,B]=C.useState(""),[V,te]=C.useState(!1),[z,Y]=C.useState(null),[ce,de]=C.useState("flashcards"),[_e,F]=C.useState(()=>k5()),[G,P]=C.useState(null),[ae,Se]=C.useState(""),[je,we]=C.useState(null),[ve,xe]=C.useState(""),[ke,me]=C.useState([]),Ge=C.useRef(null),[Ke,rt]=C.useState("all"),[nt,qe]=C.useState(null),[Xt,Pt]=C.useState(null);C.useEffect(()=>{e&&e!==p&&_(e)},[e,p]),C.useEffect(()=>{vt(Ni,a)},[a]),C.useEffect(()=>{vt(ad,l)},[l]);const lt=C.useCallback(async le=>{try{const Te=await Me("/v1/notebooks",t,{},a);if(!Te.ok){if(Te.status===401){qe("🔐 当前网关已启用访问鉴权（HTTP 401）：请在右上角「Token」输入框填入服务端 YANSHI_API_TOKEN。");return}const Ne=await Te.text();throw new Error(`加载笔记本列表失败 (${Te.status}): ${Ne}`)}qe(Ne=>Ne&&Ne.includes("HTTP 401")?null:Ne);const Re=await tt(eO,Te,"GET /v1/notebooks");m(Re.notebooks),_(Ne=>{const Ce=le||Ne;return Ce&&Re.notebooks.some(ct=>ct.id===Ce)?Ce:Re.notebooks[0]?.id??""})}catch(Te){qe(ts(Te))}},[t,a]),St=C.useCallback(async le=>{if(!le){b(null),S([]),k([]);return}try{const[Te,Re]=await Promise.all([Me(`/v1/notebooks/${encodeURIComponent(le)}`,t,{},a),Me(`/v1/notebooks/${encodeURIComponent(le)}/artifacts`,t,{},a)]);if(!Te.ok){if(Te.status===404){b(null),S([]),k([]);return}const Ce=await Te.text();throw new Error(`读取笔记本详情失败 (${Te.status}): ${Ce}`)}const Ne=await tt(tO,Te,"GET /v1/notebooks/:id");if(b(Ne.notebook),S(Ne.sources),Re.ok){const Ce=await tt(oO,Re,"GET /v1/notebooks/:id/artifacts");k(Ce.artifacts)}}catch(Te){qe(ts(Te))}},[t,a]);C.useEffect(()=>{lt(e??void 0)},[lt,e]),C.useEffect(()=>{Y(null),St(p)},[p,St]);const mn=C.useMemo(()=>v.some(le=>le.status==="pending"||le.status==="ingesting"),[v]);C.useEffect(()=>{if(!p||!mn)return;const le=window.setInterval(()=>{St(p)},1200);return()=>window.clearInterval(le)},[p,mn,St]),C.useEffect(()=>{if(!ae)return;let le=!1;const Te=new AbortController;return Ge.current?.abort(),Ge.current=Te,(async()=>{try{const Ne=await Me(`/v1/runs/${encodeURIComponent(ae)}/events`,t,{headers:{Accept:"text/event-stream"},signal:Te.signal},a);if(Ne.ok&&Ne.body){const Ce=Ne.body.getReader(),ct=new TextDecoder;let wt="";for(;!le;){const{value:Vn,done:Ws}=await Ce.read();if(Ws)break;wt+=ct.decode(Vn,{stream:!0});const vs=Md(wt,Od,"notebook generator SSE");wt=vs.rest;for(const Nn of vs.events)me(Hs=>Hs.some(ei=>ei.seq===Nn.seq)?Hs:[...Hs,Nn])}}if(!le){const Ce=await Me(`/v1/runs/${encodeURIComponent(ae)}`,t,{},a);if(Ce.ok){const ct=await tt(Ml,Ce,"GET generator run"),wt=dr(ct);xe(wt.status),wt.status==="failed"&&wt.error?qe(`生成任务失败：${wt.error}`):wt.status==="succeeded"&&Pt("成品已生成并自动挂载至右侧画廊。")}p&&await St(p)}}catch(Ne){if(le||Ne instanceof DOMException&&Ne.name==="AbortError")return;qe(ts(Ne))}})(),()=>{le=!0,Te.abort()}},[ae,t,a,p,St]);const Tt=C.useMemo(()=>v.filter(le=>le.status==="ready").length,[v]),Ft=C.useMemo(()=>{const le=new Map;for(const Te of v)le.set(Te.id,Te.title);return le},[v]),Jt=C.useMemo(()=>Nk(ke),[ke]),zn=le=>{_(le),gs(le?`/ui/notebooks/${encodeURIComponent(le)}`:"/ui/notebooks")},zi=async le=>{le.preventDefault();const Te=j.trim();if(!Te){qe("请输入笔记本标题后再创建。");return}T(!0),qe(null);try{const Re=await Me("/v1/notebooks",t,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({title:Te})},a);if(!Re.ok){const Ce=await Re.text();throw new Error(`创建笔记本失败 (${Re.status}): ${Ce}`)}const Ne=await tt(Rh,Re,"POST /v1/notebooks");A(""),await lt(Ne.id),zn(Ne.id),Pt(`已创建笔记本「${Ne.title}」，请添加资料来源。`)}catch(Re){qe(ts(Re))}finally{T(!1)}},Us=async le=>{if(le.preventDefault(),!p){qe("请先创建或选择一个笔记本。");return}D(!0),qe(null);try{let Te;if(E==="text"){if(!Z.trim())throw new Error("粘贴的资料正文不能为空。");Te={kind:"text",text:Z,...R.trim()?{title:R.trim()}:{}}}else if(E==="url"){const Ce=Q.trim();if(!/^https?:\/\//i.test(Ce))throw new Error("来源 URL 必须以 http:// 或 https:// 开头。");Te={kind:"url",url:Ce,...R.trim()?{title:R.trim()}:{}}}else{if(!X)throw new Error("请选择一个 .txt、.md 或 .html 文件。");const Ce=await X.arrayBuffer(),ct=x5(new Uint8Array(Ce));Te={kind:"file",filename:X.name,content_base64:ct}}const Re=await Me(`/v1/notebooks/${encodeURIComponent(p)}/sources`,t,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify(Te)},a);if(!Re.ok){const Ce=await Re.text();throw new Error(`添加来源失败 (${Re.status}): ${Ce}`)}const Ne=await tt(vw,Re,"POST /v1/notebooks/:id/sources");S(Ce=>[...Ce,Ne]),q(""),E==="text"&&oe(""),E==="url"&&ee(""),E==="file"&&L(null),Pt(`来源「${Ne.title}」已提交后台分块与索引构建。`)}catch(Te){qe(ts(Te))}finally{D(!1)}},Dn=async le=>{if(le.preventDefault(),!p)return;const Te=U.trim();if(!Te){Y(null);return}te(!0),qe(null);try{const Re=await Me(`/v1/notebooks/${encodeURIComponent(p)}/search?q=${encodeURIComponent(Te)}&top_k=6`,t,{},a);if(!Re.ok){const Ce=await Re.text();throw new Error(`BM25 检索失败 (${Re.status}): ${Ce}`)}const Ne=await tt(sO,Re,"GET /v1/notebooks/:id/search");Y(Ne.chunks)}catch(Re){qe(ts(Re))}finally{te(!1)}},Qt=le=>{const Te=le.replace(/\s+/g," ").trim().slice(0,140);F(Re=>({...Re,[ce]:{...Re[ce],focus:Te}})),Pt(`已将该段落设为「${ce}」生成器的聚焦主题。`)},Be=(le,Te,Re)=>{F(Ne=>({...Ne,[le]:{...Ne[le],[Te]:Re}}))},Mt=async le=>{if(!p){qe("请先创建或选择一个笔记本。");return}if(Tt===0){qe("当前笔记本尚无「已就绪」资料分块，请先添加并等待资料入库完成。");return}const Te=_e[le.kind]??{},Re={};for(const Ne of le.fields){const Ce=Te[Ne.key];if(Ne.type==="checkbox")Re[Ne.key]=!!Ce;else if(Ne.type==="number"){const ct=Number(String(Ce??Ne.defaultValue).trim());if(!Number.isFinite(ct)){qe(`参数「${Ne.label}」必须是有效数字。`);return}Re[Ne.key]=ct}else{const ct=String(Ce??Ne.defaultValue).trim();ct&&(Re[Ne.key]=ct)}}P(le.kind),qe(null),Pt(null);try{const Ne={kind:le.kind,params:Re};l.trim()&&(Ne.model=l.trim());const Ce=await Me(`/v1/notebooks/${encodeURIComponent(p)}/generate`,t,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify(Ne)},a);if(!Ce.ok){const wt=await Ce.text();throw new Error(`启动生成器失败 (${Ce.status}): ${wt}`)}const ct=await tt(iO,Ce,"POST /v1/notebooks/:id/generate");me([]),we(le.kind),xe(ct.status??"running"),Se(ct.run_id)}catch(Ne){qe(ts(Ne))}finally{P(null)}},Bs=C.useMemo(()=>{const le=[...w].reverse();return Ke==="all"?le:le.filter(Te=>Te.kind===Ke)},[w,Ke]),va=C.useMemo(()=>Object.values(Jt.telemetryByNode).filter(le=>le.status==="succeeded"||le.status==="skipped").length,[Jt.telemetryByNode]);return d.jsxs("div",{className:"nb-shell","data-testid":"notebook-workspace-page",children:[d.jsx(_a,{eyebrow:"Yanshi Content Studio",title:"多源内容笔记本工作台",subtitle:"汇集网页、文档与笔记分块构建本地 BM25 知识索引，一键驱动 8 套溯源多模态生成器",activeSection:"notebooks",actions:d.jsxs("div",{className:"nb-header-controls",children:[d.jsxs("label",{className:"nb-inline-control",htmlFor:"nb-tenant-input",children:[d.jsx("span",{children:"租户"}),d.jsx("input",{id:"nb-tenant-input",name:"tenant",type:"text",autoCorrect:"off",autoCapitalize:"none",spellCheck:!1,value:a,onChange:le=>o(le.target.value)})]}),d.jsxs("label",{className:"nb-inline-control",htmlFor:"nb-token-input",children:[d.jsx("span",{children:"Token"}),d.jsx("input",{id:"nb-token-input",name:"token",type:"password",autoComplete:"off",placeholder:"可选 Bearer",value:t,onChange:le=>s(le.target.value)})]})]})}),nt&&d.jsxs("div",{className:"studio-alert studio-alert--error",role:"alert",children:[d.jsx("span",{children:nt}),d.jsx("button",{type:"button",onClick:()=>qe(null),children:"关闭"})]}),Xt&&d.jsxs("div",{className:"studio-alert studio-alert--info",role:"status",children:[d.jsx("span",{children:Xt}),d.jsx("button",{type:"button",onClick:()=>Pt(null),children:"知道了"})]}),d.jsxs("main",{className:"nb-workbench",children:[d.jsxs("aside",{className:"nb-column nb-column--sources","aria-label":"笔记本与资料来源库",children:[d.jsxs("section",{className:"nb-card",children:[d.jsxs("div",{className:"nb-card__head",children:[d.jsx("h2",{id:"nb-picker-heading",children:"当前笔记本"}),d.jsxs("span",{children:[f.length," 本"]})]}),f.length>0&&d.jsxs("div",{className:"nb-field",children:[d.jsx("label",{htmlFor:"nb-select",children:"切换已有笔记本"}),d.jsx("select",{id:"nb-select",name:"notebook_id","data-testid":"notebook-selector",value:p,onChange:le=>zn(le.target.value),children:f.map(le=>d.jsx("option",{value:le.id,children:le.title},le.id))})]}),d.jsxs("form",{className:"nb-create-form",method:"POST",action:"/v1/notebooks","aria-labelledby":"nb-picker-heading",onSubmit:le=>{zi(le)},children:[d.jsx("label",{htmlFor:"nb-new-title",children:"新建主题笔记本"}),d.jsxs("div",{className:"nb-create-form__row",children:[d.jsx("input",{id:"nb-new-title",name:"title",type:"text","data-testid":"new-notebook-title-input",placeholder:"例如：Rust 异步运行时调研",value:j,onChange:le=>A(le.target.value)}),d.jsx("button",{type:"submit",className:"studio-btn-primary studio-btn-sm","data-testid":"create-notebook-btn",disabled:N,children:N?"创建中…":"新建"})]})]})]}),d.jsxs("section",{className:"nb-card",children:[d.jsxs("div",{className:"nb-card__head",children:[d.jsx("h2",{id:"nb-ingest-heading",children:"添加资料来源"}),d.jsxs("span",{children:[Tt,"/",v.length," 已就绪"]})]}),d.jsxs("div",{className:"nb-tabs",role:"tablist","aria-label":"资料导入方式",children:[d.jsx("button",{type:"button",role:"tab",id:"nb-tab-text","aria-selected":E==="text",onClick:()=>O("text"),children:"粘贴文本"}),d.jsx("button",{type:"button",role:"tab",id:"nb-tab-url","aria-selected":E==="url",onClick:()=>O("url"),children:"网页 URL"}),d.jsx("button",{type:"button",role:"tab",id:"nb-tab-file","aria-selected":E==="file",onClick:()=>O("file"),children:"上传文档"})]}),d.jsxs("form",{className:"nb-ingest-form",method:"POST",action:"/v1/notebooks/sources","aria-labelledby":"nb-ingest-heading",onSubmit:le=>{Us(le)},children:[d.jsx("div",{role:"tabpanel","aria-labelledby":"nb-tab-text",hidden:E!=="text",children:d.jsxs("div",{className:"nb-tabpanel-inner",children:[d.jsxs("div",{className:"nb-field",children:[d.jsx("label",{htmlFor:"nb-src-text-title",children:"来源标题（可选）"}),d.jsx("input",{id:"nb-src-text-title",name:"text_title",type:"text",placeholder:"留空自动截取首句作为标题",value:R,onChange:le=>q(le.target.value)})]}),d.jsxs("div",{className:"nb-field",children:[d.jsx("label",{htmlFor:"nb-src-text-body",children:"资料正文（自动按段落切块）"}),d.jsx("textarea",{id:"nb-src-text-body",name:"text",rows:5,"data-testid":"source-text-input",placeholder:"粘贴论文摘录、会议纪要、架构说明或课堂笔记…",value:Z,onChange:le=>oe(le.target.value)})]})]})}),d.jsx("div",{role:"tabpanel","aria-labelledby":"nb-tab-url",hidden:E!=="url",children:d.jsxs("div",{className:"nb-tabpanel-inner",children:[d.jsxs("div",{className:"nb-field",children:[d.jsx("label",{htmlFor:"nb-src-url",children:"网页链接（内置 SSRF 安全抓取与正文提取）"}),d.jsx("input",{id:"nb-src-url",name:"url",type:"url",autoCorrect:"off",autoCapitalize:"none",spellCheck:!1,placeholder:"https://example.org/article",value:Q,onChange:le=>ee(le.target.value)})]}),d.jsxs("div",{className:"nb-field",children:[d.jsx("label",{htmlFor:"nb-src-url-title",children:"备注标题（可选）"}),d.jsx("input",{id:"nb-src-url-title",name:"url_title",type:"text",placeholder:"留空默认使用 URL",value:R,onChange:le=>q(le.target.value)})]})]})}),d.jsx("div",{role:"tabpanel","aria-labelledby":"nb-tab-file",hidden:E!=="file",children:d.jsx("div",{className:"nb-tabpanel-inner",children:d.jsxs("div",{className:"nb-field",children:[d.jsx("label",{htmlFor:"nb-src-file",children:"选择本地文档（支持 .md / .txt / .html，≤10MB）"}),d.jsx("input",{id:"nb-src-file",name:"file",type:"file",accept:".txt,.md,.markdown,.html,.htm,text/plain,text/markdown,text/html",onChange:le=>L(le.target.files?.[0]??null)})]})})}),d.jsx("button",{type:"submit",className:"studio-btn-primary","data-testid":"submit-source-btn",disabled:I||!p,children:I?"正在提交入库…":"+ 加入当前笔记本"})]}),d.jsx("div",{className:"nb-source-list-wrap",children:v.length===0?d.jsx("p",{className:"nb-empty-hint",children:y?"尚未添加资料。导入文本、网页或 Markdown/HTML 文件后即可检索和生成成品。":"请先在上方新建一个主题笔记本。"}):d.jsx("ul",{className:"nb-source-list","aria-label":"已添加的资料来源",children:v.map(le=>d.jsxs("li",{className:"nb-source-item",children:[d.jsxs("div",{className:"nb-source-item__top",children:[d.jsx("strong",{children:le.title}),d.jsx("span",{className:"nb-source-status","data-status":le.status,children:j5(le.status)})]}),d.jsxs("div",{className:"nb-source-item__meta",children:[d.jsx("span",{children:le.kind.toUpperCase()}),d.jsx("span",{children:Zn(le.created_at)}),le.sha256?d.jsxs("code",{children:["#",le.sha256.slice(0,8)]}):null]}),le.error?d.jsx("p",{className:"nb-source-item__error",children:le.error}):null]},le.id))})})]}),d.jsxs("section",{className:"nb-card",children:[d.jsx("div",{className:"nb-card__head",children:d.jsx("h2",{id:"nb-search-heading",children:"BM25 段落溯源检索"})}),d.jsx("search",{children:d.jsxs("form",{className:"nb-search-form",method:"GET",action:"/v1/notebooks/search","aria-labelledby":"nb-search-heading",onSubmit:le=>{Dn(le)},children:[d.jsx("label",{htmlFor:"nb-search-q",className:"nb-sr-only",children:"检索笔记本原文分块"}),d.jsx("input",{id:"nb-search-q",name:"q",type:"search","data-testid":"notebook-search-input",autoCorrect:"off",autoCapitalize:"none",spellCheck:!1,placeholder:"输入关键词检索原文分块…",value:U,onChange:le=>B(le.target.value)}),d.jsx("button",{type:"submit",className:"studio-btn-secondary studio-btn-sm",disabled:V||!p,children:V?"检索中…":"检索"})]})}),z!==null&&d.jsx("div",{className:"nb-search-results",children:z.length===0?d.jsx("p",{className:"nb-empty-hint",children:"未找到匹配分块，请尝试调整关键词。"}):d.jsx("ul",{className:"nb-hit-list",children:z.map(le=>d.jsxs("li",{className:"nb-hit-card",children:[d.jsxs("div",{className:"nb-hit-card__head",children:[d.jsx("span",{children:Ft.get(le.source_id)??"资料分块"}),d.jsxs("code",{children:["BM25 ",le.score.toFixed(2)]})]}),d.jsx("p",{children:le.content}),d.jsxs("div",{className:"nb-hit-card__foot",children:[d.jsx("small",{children:le.chunk_id}),d.jsx("button",{type:"button",className:"studio-link-btn",onClick:()=>Qt(le.content),children:"设为生成焦点"})]})]},le.chunk_id))})})]})]}),d.jsxs("section",{className:"nb-column nb-column--studio","aria-label":"Content Studio 生成器矩阵",children:[d.jsxs("div",{className:"nb-studio-bar",children:[d.jsxs("div",{children:[d.jsx("h2",{children:"Content Studio 生成器工作台"}),d.jsx("p",{children:y?`基于「${y.title}」的 ${Tt} 份就绪资料原文，生成带分块引用的结构化交付物`:"创建并选择笔记本后即可一键生成多模态成品"})]}),d.jsxs("div",{className:"nb-model-box",children:[d.jsx("label",{htmlFor:"nb-model-input",children:"生成模型 (model)"}),d.jsx("input",{id:"nb-model-input",name:"model",type:"text","data-testid":"notebook-model-input",autoCorrect:"off",autoCapitalize:"none",spellCheck:!1,placeholder:"例如 gpt-4o / kimi-k2.6（留空读 YANSHI_GENERATE_MODEL）",value:l,onChange:le=>c(le.target.value)})]})]}),ae&&d.jsxs("div",{className:"nb-live-run-ribbon",role:"status","data-testid":"notebook-live-run",children:[d.jsxs("div",{className:"nb-live-run-ribbon__info",children:[d.jsx("span",{className:`nb-run-dot ${["succeeded","failed","cancelled"].includes(ve)?"nb-run-dot--done":"nb-run-dot--running"}`}),d.jsxs("strong",{children:[je??"workflow"," · 状态：",ve||"running"]}),d.jsxs("span",{children:["已完成 ",va," 个节点"]}),Jt.totalCostUsd>0&&d.jsxs("span",{className:"studio-spend-pill",children:["💰 ",Xa(Jt.totalCostUsd)]})]}),d.jsx("div",{className:"nb-live-run-ribbon__actions",children:d.jsx(ln,{className:"studio-btn-secondary studio-btn-sm",to:`/ui/studio?run_id=${encodeURIComponent(ae)}${je?`&preset=${encodeURIComponent(je)}`:""}`,children:"在 Studio 拓扑台中查看 / 续跑"})})]}),d.jsx("div",{className:"nb-generator-grid",children:wh.map(le=>{const Te=ce===le.kind,Re=G===le.kind,Ne=_e[le.kind]??{};return d.jsxs("article",{className:`nb-gen-card ${Te?"nb-gen-card--expanded":""}`,"data-testid":`generator-card-${le.kind}`,children:[d.jsxs("div",{className:"nb-gen-card__head",children:[d.jsxs("div",{children:[d.jsx("span",{className:"nb-gen-badge",children:le.deliverableBadge}),d.jsx("h3",{children:le.title})]}),d.jsx("button",{type:"button",className:"studio-link-btn","aria-expanded":Te,onClick:()=>de(le.kind),children:Te?"参数已展开":"调整参数"})]}),d.jsx("p",{className:"nb-gen-card__desc",children:le.summary}),Te&&d.jsx("div",{className:"nb-gen-params",children:le.fields.map(Ce=>{const ct=`nb-gen-${le.kind}-${Ce.key}`,wt=Ne[Ce.key]??Ce.defaultValue;return Ce.type==="checkbox"?d.jsxs("div",{className:"nb-gen-checkbox",children:[d.jsx("input",{id:ct,name:Ce.key,type:"checkbox",checked:!!wt,onChange:Vn=>Be(le.kind,Ce.key,Vn.target.checked)}),d.jsx("label",{htmlFor:ct,children:Ce.label})]},Ce.key):d.jsxs("div",{className:"nb-field",children:[d.jsx("label",{htmlFor:ct,children:Ce.label}),d.jsx("input",{id:ct,name:Ce.key,type:Ce.type==="number"?"number":"text",min:Ce.min,max:Ce.max,value:String(wt),onChange:Vn=>Be(le.kind,Ce.key,Vn.target.value)})]},Ce.key)})}),d.jsxs("div",{className:"nb-gen-card__foot",children:[d.jsx(ln,{className:"studio-link-btn",to:`/ui/studio?preset=${encodeURIComponent(le.kind)}`,children:"打开 YAML 编排源码"}),d.jsx("button",{type:"button",className:"studio-btn-primary studio-btn-sm","data-testid":`launch-generator-${le.kind}`,disabled:Re||!p||Tt===0,onClick:()=>{Mt(le)},children:Re?"提交中…":"✨ 生成成品"})]})]},le.kind)})})]}),d.jsx("aside",{className:"nb-column nb-column--gallery","aria-label":"笔记本多模态成品画廊",children:d.jsxs("div",{className:"nb-card nb-card--gallery",children:[d.jsxs("div",{className:"nb-card__head",children:[d.jsxs("h2",{children:["成品画廊 (",Bs.length,")"]}),d.jsx("button",{type:"button",className:"studio-link-btn",disabled:!p,onClick:()=>{St(p)},children:"刷新"})]}),d.jsxs("div",{className:"nb-gallery-filters",role:"group","aria-label":"按生成类型筛选成品",children:[d.jsxs("button",{type:"button",className:Ke==="all"?"active":"",onClick:()=>rt("all"),children:["全部 (",w.length,")"]}),wh.map(le=>{const Te=w.filter(Re=>Re.kind===le.kind).length;return Te===0&&Ke!==le.kind?null:d.jsxs("button",{type:"button",className:Ke===le.kind?"active":"",onClick:()=>rt(le.kind),children:[le.title," (",Te,")"]},le.kind)})]}),d.jsx("div",{className:"nb-gallery-list",children:Bs.length===0?d.jsxs("div",{className:"nb-empty-gallery",children:[d.jsx("h3",{children:"尚无已生成的交付物"}),d.jsx("p",{children:"在左侧导入至少一份资料并等待状态变为「已就绪」，即可在中间点击任意生成器产出闪卡、导图、报告、课件或音视频。"})]}):Bs.map(le=>{const Te=S5(le);return d.jsxs("div",{className:"nb-artifact-block",children:[d.jsxs("div",{className:"nb-artifact-block__meta",children:[d.jsx("span",{className:"nb-gen-badge",children:le.kind}),d.jsx("small",{children:Zn(le.created_at)}),d.jsxs(ln,{className:"studio-link-btn",to:`/ui/studio?run_id=${encodeURIComponent(le.run_id)}&preset=${encodeURIComponent(le.kind)}`,children:["查看运行 #",le.run_id.slice(0,8)]})]}),d.jsx(ql,{item:Te,apiToken:t,tenantId:a})]},le.id)})})]})})]})]})}const N5=`name: my-new-run
nodes:
  - id: greet
    type: template
    parameters:
      template: "hello {{ name }}"`,E5=`{
  "name": "world"
}`,Ox=["dev","local","production"],T5=e=>{const t=e.trim();if(!t)return{ok:!0,value:null};try{return{ok:!0,value:JSON.parse(t)}}catch(s){return{ok:!1,error:s instanceof Error?s.message:String(s)}}},Mx=e=>Math.max(1,e.split(`
`).length);function C5({apiToken:e,onTokenChange:t}){const[s,a]=C.useState(()=>bt(Iv,"default")),[o,l]=C.useState(()=>{const T=bt(Zv,"local");return Ox.includes(T)?T:"local"}),[c,f]=C.useState(()=>bt(Hv,N5)),[m,p]=C.useState(()=>bt(Pv,E5)),[_,y]=C.useState(null),[b,v]=C.useState(!1),[S,w]=C.useState(null);C.useEffect(()=>{vt(Iv,s)},[s]),C.useEffect(()=>{vt(Zv,o)},[o]),C.useEffect(()=>{vt(Hv,c)},[c]),C.useEffect(()=>{vt(Pv,m)},[m]);const k=C.useMemo(()=>{const T=c.trim();return T?/^\s*name\s*:/m.test(T)?/^\s*nodes\s*:/m.test(T)?{ok:!0,message:"looks like a workflow YAML (basic checks)"}:{ok:!1,message:"workflow must define a top-level 'nodes:' map"}:{ok:!1,message:"workflow must define a top-level 'name:' field"}:{ok:!1,message:"workflow YAML is required"}},[c]),j=C.useMemo(()=>T5(m),[m]),A=async(T,E)=>{const O=T.target.files?.[0];if(O)try{const R=await O.text();E==="workflow"?f(R):p(R),w(`loaded ${O.name} into ${E}`)}catch(R){y(R instanceof Error?R.message:String(R))}finally{T.target.value=""}},N=async T=>{if(T.preventDefault(),y(null),w(null),!k.ok){y(k.message);return}if(!j.ok){y(`inputs JSON is invalid: ${j.error}`);return}v(!0);try{const E={tenant_id:s.trim()||"default",workflow:c,profile:o};j.value&&typeof j.value=="object"&&(E.inputs=j.value);const O=E.tenant_id??"default",R=await Me("/v1/runs",e,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify(E)},O);if(!R.ok){const Z=await R.text();throw new Error(`run submission failed with HTTP ${R.status}: ${Z}`)}const q=await tt(Dh,R,"POST /v1/runs");vt(Fm,c),vt(Ni,s),gs(`/ui?run=${encodeURIComponent(q.run_id)}`)}catch(E){y(E instanceof Error?E.message:String(E))}finally{v(!1)}};return d.jsxs("main",{className:"shell create-shell",children:[d.jsx(_a,{eyebrow:"Yanshi / Runs",title:"Submit a run",activeSection:"runs-new",actions:d.jsx(ln,{className:"topbar-link",to:"/ui/console",children:"← Run console"})}),_?d.jsx("p",{className:"error-line",children:_}):null,S?d.jsx("p",{className:"info-line",children:S}):null,d.jsxs("form",{className:"create-form",onSubmit:N,children:[d.jsxs("section",{className:"create-row",children:[d.jsxs("label",{children:[d.jsx("span",{children:"Tenant"}),d.jsx("input",{"data-testid":"create-tenant",value:s,onChange:T=>a(T.target.value),placeholder:"default"})]}),d.jsxs("label",{children:[d.jsx("span",{children:"Profile"}),d.jsx("select",{"data-testid":"create-profile",value:o,onChange:T=>l(T.target.value),children:Ox.map(T=>d.jsx("option",{value:T,children:T},T))})]}),d.jsxs("label",{children:[d.jsx("span",{children:"API token"}),d.jsx("input",{"data-testid":"create-token",autoComplete:"off",type:"password",value:e,onChange:T=>t(T.target.value),placeholder:"Bearer token (not persisted)"})]})]}),d.jsxs("section",{className:"create-editor","aria-label":"Workflow YAML editor",children:[d.jsxs("div",{className:"pane-heading",children:[d.jsx("span",{children:"Workflow YAML"}),d.jsx("span",{className:`validation ${k.ok?"validation-ok":"validation-err"}`,children:k.message})]}),d.jsxs("div",{className:"editor-actions",children:[d.jsxs("label",{className:"file-pick",children:["Load from file…",d.jsx("input",{type:"file",accept:".yaml,.yml,.txt,text/yaml,application/yaml",onChange:T=>A(T,"workflow")})]}),d.jsxs("span",{className:"line-meter",children:["lines: ",Mx(c)]})]}),d.jsx(rd,{testId:"create-workflow",ariaLabel:"Workflow YAML",value:c,onChange:f,rows:18})]}),d.jsxs("section",{className:"create-editor","aria-label":"Inputs JSON editor",children:[d.jsxs("div",{className:"pane-heading",children:[d.jsx("span",{children:"Inputs (JSON, optional)"}),d.jsx("span",{className:`validation ${j.ok?"validation-ok":"validation-err"}`,children:j.ok?"valid JSON or empty":j.error})]}),d.jsxs("div",{className:"editor-actions",children:[d.jsxs("label",{className:"file-pick",children:["Load from file…",d.jsx("input",{type:"file",accept:".json,application/json",onChange:T=>A(T,"inputs")})]}),d.jsxs("span",{className:"line-meter",children:["lines: ",Mx(m)]})]}),d.jsx("textarea",{"data-testid":"create-inputs",className:"code-editor code-editor-json",spellCheck:!1,value:m,onChange:T=>p(T.target.value),rows:8})]}),d.jsxs("footer",{className:"create-actions",children:[d.jsx("button",{"data-testid":"create-submit",disabled:b||!k.ok||!j.ok,type:"submit",children:b?"Submitting…":"Submit run"}),d.jsx("small",{children:"Inputs persist in localStorage (tenant, profile, workflow, inputs). The API token is never saved."})]})]})]})}const A5=`name: web-ui-console-smoke
nodes:
  - id: hello
    type: template
    parameters:
      template: "hello from the run console"`;function Sh(e){const t=e.payload??{},s=typeof t.step_index=="number"?t.step_index:e.seq;return`${e.kind}#${s}`}function Ek(e,t){if(t===0)return 0;const s=Date.parse(e[t-1].ts),a=Date.parse(e[t].ts);return Number.isNaN(s)||Number.isNaN(a)?null:a-s}async function zx(e,t,s){const a=await Me(`/v1/runs/${encodeURIComponent(e)}/events/history?limit=1000`,t,{},s);if(!a.ok)throw new Error(`run ${e}: ${a.status} ${a.statusText}`);const o=await a.json();return UO(o)}function O5({primaryRunId:e,routeSearch:t,apiToken:s,onTokenChange:a}){const l=C.useMemo(()=>new URLSearchParams(t),[t]).get("against")??"",[c,f]=C.useState(l),[m,p]=C.useState([]),[_,y]=C.useState([]),[b,v]=C.useState([]),[S,w]=C.useState("idle"),[k,j]=C.useState(null);C.useEffect(()=>{f(l)},[l]);const A=C.useMemo(()=>new URLSearchParams(t).get("tenant")?.trim()||bt(Ni,"default"),[t]);C.useEffect(()=>{let q=!1;return Me("/v1/runs?limit=25",s,{},A).then(Z=>Z.ok?tt(zh,Z,"GET /v1/runs"):null).then(Z=>{q||!Z||p(Z.runs)}).catch(()=>{}),()=>{q=!0}},[s,A]),C.useEffect(()=>{if(!e)return;let q=!1;w("loading"),j(null);const Z=c&&c!==e?zx(c,s,A):Promise.resolve([]);return Promise.all([zx(e,s,A),Z]).then(([oe,Q])=>{q||(y(oe),v(Q),w("idle"))}).catch(oe=>{q||(j(ts(oe)),w("error"))}),()=>{q=!0}},[e,c,s,A]);const N=C.useMemo(()=>m.filter(q=>q.id!==e),[m,e]),T=C.useMemo(()=>new Set(b.map(Sh)),[b]),E=C.useMemo(()=>new Set(_.map(Sh)),[_]),O=C.useMemo(()=>Dx(_),[_]),R=C.useMemo(()=>Dx(b),[b]);return d.jsxs("main",{className:"shell run-compare",children:[d.jsx(_a,{eyebrow:"Yanshi / Runs",title:"Trace comparison",subtitle:"Side-by-side event timelines plus per-step diff highlight + hop latency.",activeSection:"compare"}),d.jsxs("header",{className:"run-compare-header",children:[d.jsxs("div",{className:"run-compare-controls",children:[d.jsxs("label",{children:["Primary run",d.jsx("input",{value:e,readOnly:!0})]}),N.length>0&&d.jsxs("label",{children:["Quick pick recent run",d.jsxs("select",{"aria-label":"Quick pick recent run",value:N.some(q=>q.id===c)?c:"",onChange:q=>{const Z=q.target.value;f(Z),Z&&gs(eo(e,Z))},children:[d.jsx("option",{value:"",children:"-- 选择要对比的历史运行 --"}),N.map(q=>d.jsxs("option",{value:q.id,children:[q.id.slice(0,8)," · ",q.status," · ",Zn(q.started_at)]},q.id))]})]}),d.jsxs("label",{children:["Compare against",d.jsx("input",{value:c,onChange:q=>f(q.target.value.trim()),placeholder:"run id"})]}),d.jsxs("label",{children:["API token (optional)",d.jsx("input",{type:"password",value:s,onChange:q=>a(q.target.value),placeholder:"bearer",autoComplete:"off"})]}),d.jsx("button",{type:"button",onClick:()=>gs(eo(e,c)),disabled:!c||c===e,children:"Compare"}),c&&c!==e&&d.jsx("button",{type:"button",onClick:()=>gs(eo(c,e)),children:"⇄ Swap"})]}),S==="loading"&&d.jsx("p",{className:"run-compare-status",children:"Loading…"}),k&&d.jsx("p",{className:"run-compare-error",role:"alert",children:k})]}),d.jsxs("section",{className:"run-compare-summary","aria-label":"Per-run summary",children:[d.jsx($x,{title:"Primary",runId:e,summary:O}),d.jsx($x,{title:"Compared",runId:c,summary:R})]}),d.jsxs("section",{className:"run-compare-grid","aria-label":"Event timelines",children:[d.jsx(Rx,{title:`Primary · ${e}`,events:_,otherKeys:T}),d.jsx(Rx,{title:`Compared · ${c||"—"}`,events:b,otherKeys:E})]})]})}function Dx(e){let t=0,s=0;for(let l=1;l<e.length;l+=1){const c=Ek(e,l);c!==null&&(t+=c,s+=1)}const a=e.filter(l=>l.kind.toLowerCase().includes("tool_call")).length,o=(()=>{const l=[...e].reverse().find(m=>{const p=m.kind.toLowerCase();return p==="run_completed"||p==="final_answer"||p==="stopped"});if(!l)return null;const c=l.payload??{},f=c.answer??c.final_answer??c.message;return typeof f=="string"?f:null})();return{eventCount:e.length,toolCallCount:a,finalAnswer:o,totalLatencyMs:t,meanLatencyMs:s>0?Math.round(t/s):0}}function $x({title:e,runId:t,summary:s}){return d.jsxs("article",{className:"run-compare-card",children:[d.jsx("h2",{children:e}),d.jsx("p",{className:"run-compare-card-id",children:t||"—"}),d.jsxs("dl",{children:[d.jsxs("div",{children:[d.jsx("dt",{children:"Events"}),d.jsx("dd",{children:s.eventCount})]}),d.jsxs("div",{children:[d.jsx("dt",{children:"Tool calls"}),d.jsx("dd",{children:s.toolCallCount})]}),d.jsxs("div",{children:[d.jsx("dt",{children:"Total wall-clock"}),d.jsxs("dd",{children:[s.totalLatencyMs," ms"]})]}),d.jsxs("div",{children:[d.jsx("dt",{children:"Mean hop"}),d.jsxs("dd",{children:[s.meanLatencyMs," ms"]})]})]}),s.finalAnswer&&d.jsxs("p",{className:"run-compare-final",title:"final answer",children:[d.jsx("strong",{children:"Final:"})," ",s.finalAnswer]})]})}function Rx({title:e,events:t,otherKeys:s}){return d.jsxs("div",{className:"run-compare-column","aria-label":e,children:[d.jsx("h3",{children:e}),t.length===0?d.jsx("p",{className:"run-compare-empty",children:"No events."}):d.jsx("ol",{className:"run-compare-events",children:t.map((a,o)=>{const l=Sh(a),c=s.has(l),f=Ek(t,o);return d.jsxs("li",{className:`run-compare-event ${c?"matched":"unmatched"}`,children:[d.jsxs("div",{className:"run-compare-event-row",children:[d.jsx("span",{className:`dot dot-${zl(a.kind)}`}),d.jsx("span",{className:"run-compare-event-kind",children:a.kind}),f!==null&&d.jsxs("span",{className:"run-compare-event-latency",children:["+",f," ms"]})]}),!c&&d.jsx("span",{className:"run-compare-event-tag",title:"present only in this run",children:"only here"})]},a.seq)})})]})}function M5({apiToken:e,onTokenChange:t}){const[s,a]=C.useState(""),[o,l]=C.useState(()=>bt(Ni,"default")),[c,f]=C.useState(()=>bt(Fm,A5)),m=_O(e,o),[p,_]=C.useState([]),[y,b]=C.useState(null),[v,S]=C.useState([]),[w,k]=C.useState(null),[j,A]=C.useState(""),[N,T]=C.useState("idle"),[E,O]=C.useState(null),[R,q]=C.useState("idle"),[Z,oe]=C.useState(null),Q=C.useRef(null),ee=C.useRef(-1),X=C.useRef(0),L=C.useMemo(()=>v.find(G=>G.seq===w)??v.at(-1)??null,[v,w]);C.useEffect(()=>{if(!s){A("");return}A(bt(`${ym}${s}`,""))},[s]),C.useEffect(()=>{s&&vt(`${ym}${s}`,j)},[s,j]);const I=C.useMemo(()=>HN(j),[j]),D=C.useMemo(()=>I.predicate?ZN(v,I):v,[v,I]),U=C.useMemo(()=>{const G=new Map;for(const P of v){const ae=bm(P)||P.kind;G.set(ae,{name:ae,label:ae,status:P.kind,tone:zl(P.kind)})}return Array.from(G.values()).slice(-12)},[v]),B=C.useMemo(()=>{if(!L)return null;const G=bm(L);return G?{id:G,label:G,status:L.kind,event:L}:null},[L]),V=C.useMemo(()=>v.filter(G=>{const P=zl(G.kind);return P==="agent"||P==="tool"}),[v]),te=C.useMemo(()=>v.filter(G=>{const P=G.payload;return P.provider||P.model||G.kind.toLowerCase().includes("llm")}),[v]),z=Uh();C.useEffect(()=>{const P=new URLSearchParams(z.search).get("run");P&&P!==s&&(a(P),T("loading"))},[z.search]),C.useEffect(()=>{vt(Ni,o),m.syncToServer(Ni,o)},[o,m]),C.useEffect(()=>{const G=m.serverPrefs?.["ui.run-console.tenant"];typeof G=="string"&&G!==o&&l(G)},[m.serverPrefs]),C.useEffect(()=>{vt(Fm,c)},[c]);const Y=async(G=o)=>{const P=G.trim()||"default",ae=await Me(`/v1/runs?tenant_id=${encodeURIComponent(P)}&limit=20`,e,{},P);if(!ae.ok)throw new Error(`run list failed with HTTP ${ae.status}`);const Se=await tt(zh,ae,"GET /v1/runs");_(Se.runs),!s&&Se.runs[0]&&(a(Se.runs[0].id),T("loading"))};C.useEffect(()=>{let G=!1,P=!1;const ae=async()=>{if(!(P||G)){G=!0;try{await Y()}catch{}finally{G=!1}}};ae();const Se=window.setInterval(()=>{ae()},4e3);return()=>{P=!0,window.clearInterval(Se)}},[e,o]),C.useEffect(()=>{if(!s||N!=="loading")return;let G=!1,P;Q.current?.abort(),ee.current=-1,X.current=0;const ae=ve=>{S(xe=>xe.some(ke=>ke.seq===ve.seq)?xe:[...xe,ve].sort((ke,me)=>ke.seq-me.seq)),k(xe=>xe??ve.seq),ve.seq>ee.current&&(ee.current=ve.seq)},Se=async ve=>{const xe=new AbortController;Q.current=xe;const ke=await Me(`/v1/runs/${s}/events?after_seq=${encodeURIComponent(String(ve))}`,e,{signal:xe.signal},o.trim()||"default");if(!ke.ok||!ke.body)throw new Error(`event stream failed with HTTP ${ke.status}`);T("streaming"),X.current=0;const me=ke.body.pipeThrough(new TextDecoderStream).getReader();let Ge="";for(;!G;){const{done:Ke,value:rt}=await me.read();if(Ke)break;Ge+=rt;const nt=Md(Ge,Od,"workflow SSE event");Ge=nt.rest;for(const qe of nt.events)ae(qe)}},je=()=>{if(G)return;const ve=X.current;X.current=ve+1;const xe=Ew(ve);oe(Zn(new Date().toISOString())),T("reconnecting"),P=window.setTimeout(()=>{G||Se(ee.current).catch(ke=>{G||ke instanceof DOMException&&ke.name==="AbortError"||(O(ke instanceof Error?ke.message:String(ke)),je())})},xe)};return(async()=>{const ve=o.trim()||"default";try{const xe=await Me(`/v1/runs/${s}`,e,{},ve);if(!xe.ok)throw new Error(`run lookup failed with HTTP ${xe.status}`);const ke=await tt(Ml,xe,`GET /v1/runs/${s}`);if(G)return;const me=dr(ke);b(me),S([]),k(null),O(null),gs(`/ui/console?run=${encodeURIComponent(s)}`,{replace:!0});let Ge=`/v1/runs/${s}/events/history`;const Ke=bt(`${ym}${s}`,"");Ke.trim()&&(Ge+=`?filter=${encodeURIComponent(Ke)}`);let rt=await Me(Ge,e,{},ve);rt.status===400&&Ke.trim()&&(rt=await Me(`/v1/runs/${s}/events/history`,e,{},ve));let nt=-1;if(rt.ok){const qe=await tt($h,rt,`GET /v1/runs/${s}/events/history`);S(qe),k(qe.at(-1)?.seq??null),nt=qe.at(-1)?.seq??-1,ee.current=nt}await Se(nt),G||(T("closed"),Me(`/v1/runs/${s}`,e,{},ve).then(qe=>qe.ok?qe.json():null).then(qe=>{qe&&b(dr(qe))}))}catch(xe){if(G||xe instanceof DOMException&&xe.name==="AbortError")return;O(ts(xe)),je()}})(),()=>{G=!0,Q.current?.abort(),P&&window.clearTimeout(P)}},[s,N,e]);const ce=G=>{G.preventDefault(),s.trim()&&(O(null),T("loading"))},de=async G=>{if(G.preventDefault(),!c.trim()){O("workflow YAML is required");return}q("submitting"),O(null);const P=o.trim()||"default";try{const ae=await Me("/v1/runs",e,{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify({tenant_id:P,workflow:c})},P);if(!ae.ok){const je=await ae.text();throw new Error(`run submission failed with HTTP ${ae.status}: ${je}`)}const Se=await tt(Dh,ae,"POST /v1/runs (resubmit)");a(Se.run_id),T("loading"),await Y(P)}catch(ae){O(ts(ae))}finally{q("idle")}},_e=async()=>{if(!y||cd(y))return;q("cancelling"),O(null);const G=y.tenant_id?.trim()||o.trim()||"default";try{const P=await Me(`/v1/runs/${y.id}:cancel`,e,{method:"POST"},G);if(!P.ok)throw new Error(`run cancellation failed with HTTP ${P.status}`);const ae=await tt(hw,P,`POST /v1/runs/${y.id}:cancel`);b(ae.run),T("closed"),Q.current?.abort(),await Y(ae.run.tenant_id??G)}catch(P){O(ts(P))}finally{q("idle")}},F=()=>{s.trim()&&T("loading")};return d.jsxs("main",{className:"shell",children:[d.jsx(_a,{eyebrow:"Yanshi / Classic",title:"Run Console",activeSection:"console",actions:d.jsxs("form",{className:"run-form",onSubmit:ce,children:[d.jsx("input",{"aria-label":"Run ID",value:s,onChange:G=>a(G.target.value),placeholder:"Run ID"}),d.jsx("button",{type:"submit",children:"Connect"}),d.jsx("button",{disabled:!y||cd(y)||R==="cancelling",type:"button",onClick:_e,children:"Cancel"}),s.trim()&&d.jsx(ln,{className:"topbar-link",to:eo(s.trim(),p.find(G=>G.id!==s.trim())?.id),children:"⇄ Compare"}),d.jsx(ln,{className:"topbar-link topbar-cta",to:"/ui/runs/new",children:"+ New run"})]})}),d.jsxs("section",{className:"status-strip","aria-label":"Run status",children:[d.jsxs("div",{children:[d.jsx("span",{children:"Stream"}),d.jsx("strong",{children:N})]}),d.jsxs("div",{children:[d.jsx("span",{children:"Status"}),d.jsx("strong",{children:y?.status??"none"})]}),d.jsxs("div",{children:[d.jsx("span",{children:"Tenant"}),d.jsx("strong",{children:y?.tenant_id??o})]}),d.jsxs("div",{children:[d.jsx("span",{children:"Events"}),d.jsx("strong",{children:v.length})]}),d.jsxs("div",{children:[d.jsx("span",{children:"Reconnect"}),d.jsx("strong",{children:Z??"none"})]})]}),E?d.jsx("p",{className:"error-line",children:E}):null,d.jsxs("section",{className:"workspace",children:[d.jsxs("aside",{className:"run-pane",children:[d.jsxs("div",{className:"pane-heading",children:[d.jsx("span",{children:"Runs"}),d.jsx("button",{type:"button",onClick:F,children:"Refresh"})]}),d.jsx("ol",{className:"run-list",children:p.map(G=>d.jsx("li",{children:d.jsxs("button",{className:G.id===s?"selected":"",type:"button",onClick:()=>{a(G.id),T("loading")},children:[d.jsx("span",{children:G.workflow.split(`
`)[0]||G.id}),d.jsxs("small",{children:[G.status," · ",Zn(G.started_at)]})]})},G.id))}),d.jsxs("form",{className:"submission-form",onSubmit:de,children:[d.jsxs("label",{children:[d.jsx("span",{children:"Tenant"}),d.jsx("input",{value:o,onChange:G=>l(G.target.value)})]}),d.jsxs("label",{children:[d.jsx("span",{children:"API token"}),d.jsx("input",{autoComplete:"off",type:"password",value:e,onChange:G=>t(G.target.value),placeholder:"Bearer token"})]}),d.jsxs("div",{className:"workflow-field",children:[d.jsx("span",{children:"Workflow YAML"}),d.jsx(rd,{ariaLabel:"Workflow YAML",value:c,onChange:f,minHeight:200})]}),d.jsx("button",{disabled:R==="submitting",type:"submit",children:R==="submitting"?"Submitting":"Submit run"})]})]}),d.jsxs("section",{className:"graph-pane","aria-label":"DAG status",children:[d.jsxs("div",{className:"pane-heading",children:[d.jsx("span",{children:"DAG"}),d.jsxs("strong",{children:[U.length," nodes"]})]}),d.jsx("div",{className:"node-grid",children:U.length===0?d.jsx("div",{className:"empty-node",children:"Waiting for events"}):U.map(G=>d.jsxs("button",{className:`node node-${G.tone}`,type:"button",onClick:()=>{const P=qO(v,ae=>bm(ae)===G.name);k(P?.seq??null)},children:[d.jsx("span",{children:G.label}),d.jsx("small",{children:G.status})]},G.name))})]}),d.jsxs("aside",{className:"timeline-pane","aria-label":"Agent timeline",children:[d.jsxs("div",{className:"pane-heading",children:[d.jsx("span",{children:"Timeline"}),d.jsxs("strong",{children:[L?`#${L.seq}`:"-",j.trim()&&I.predicate?` (${D.length}/${v.length})`:""]})]}),d.jsxs("div",{className:"event-filter",children:[d.jsx("label",{htmlFor:"event-filter-input",children:"Filter"}),d.jsx("input",{id:"event-filter-input",type:"text",placeholder:"kind=tool_call_completed AND step>5",value:j,onChange:G=>A(G.target.value),spellCheck:!1,autoComplete:"off"}),I.error&&d.jsx("span",{className:"event-filter-error",role:"alert",children:I.error})]}),d.jsxs("ol",{className:"timeline",children:[D.map(G=>d.jsx("li",{children:d.jsxs("button",{className:w===G.seq?"selected":"",type:"button",onClick:()=>k(G.seq),children:[d.jsx("span",{className:`dot dot-${zl(G.kind)}`}),d.jsx("span",{children:G.kind}),d.jsx("time",{children:Zn(G.ts)})]})},G.seq)),I.predicate&&D.length===0&&v.length>0&&d.jsx("li",{className:"timeline-empty",children:"No events match this filter."})]})]})]}),d.jsxs("section",{className:"details-grid","aria-label":"Run details",children:[d.jsxs("section",{className:"details-pane",children:[d.jsxs("div",{className:"pane-heading",children:[d.jsx("span",{children:"Provider / config"}),d.jsx("strong",{children:te.length?"from events":e?"token set":"open / unset"})]}),d.jsx("pre",{children:As({tenant_id:y?.tenant_id??o,run_dir:y?.run_dir??null,auth:e?"bearer token configured in browser":"no browser token configured",latest_provider_event:te.at(-1)?.payload??null})})]}),d.jsxs("section",{className:"details-pane",children:[d.jsxs("div",{className:"pane-heading",children:[d.jsx("span",{children:"DAG node"}),d.jsx("strong",{children:B?.id??"none"})]}),d.jsx("pre",{children:B?As(B):"Select a node event."})]}),d.jsxs("section",{className:"details-pane",children:[d.jsxs("div",{className:"pane-heading",children:[d.jsx("span",{children:"Agent / tool policy"}),d.jsx("strong",{children:V.at(-1)?.kind??"none"})]}),d.jsx("pre",{children:V.at(-1)?As(V.at(-1)):"No agent or tool events."})]}),d.jsxs("section",{className:"details-pane",children:[d.jsxs("div",{className:"pane-heading",children:[d.jsx("span",{children:"Event payload"}),d.jsx("strong",{children:L?.kind??"none"})]}),d.jsx("pre",{children:L?As(L.payload):"Select an event."})]})]})]})}function z5(){const e=Uh(),t=e.pathname,[s,a]=C.useState(()=>lO(Uv,""));if(C.useEffect(()=>{cO(Uv,s)},[s]),t==="/ui/notebooks"||t==="/ui/notebooks/")return d.jsx(Ax,{apiToken:s,onTokenChange:a});const o=v5(t);if(o)return d.jsx(Ax,{initialNotebookId:o,apiToken:s,onTokenChange:a});if(t==="/ui/runs/new"||t==="/ui/runs/new/")return d.jsx(C5,{apiToken:s,onTokenChange:a});if(t==="/ui/diagnostics"||t==="/ui/diagnostics/")return d.jsx(DO,{apiToken:s,onTokenChange:a});if(t==="/ui/harness/sessions"||t==="/ui/harness/sessions/")return d.jsx(PO,{apiToken:s,onTokenChange:a});if(t==="/ui/harness/sessions/new"||t==="/ui/harness/sessions/new/")return d.jsx(GO,{apiToken:s,onTokenChange:a});const l=RO(t);if(l)return d.jsx(HO,{sessionId:l,apiToken:s,onTokenChange:a});const c=t.match(/^\/ui\/runs\/([^/]+)\/compare\/?$/);return c?d.jsx(O5,{primaryRunId:decodeURIComponent(c[1]),routeSearch:e.search,apiToken:s,onTokenChange:a}):t==="/ui/console"||t==="/ui/console/"?d.jsx(M5,{apiToken:s,onTokenChange:a}):t==="/ui/app"||t==="/ui/app/"?d.jsx(Vm,{initialMode:"release",apiToken:s,onTokenChange:a}):t==="/ui/studio"||t==="/ui/studio/"?d.jsx(Vm,{initialMode:"debug",apiToken:s,onTokenChange:a}):d.jsx(Vm,{apiToken:s,onTokenChange:a})}const Lx=document.getElementById("yanshi-debugger");Lx&&qN.createRoot(Lx).render(d.jsx(bO,{children:d.jsx(z5,{})}));
