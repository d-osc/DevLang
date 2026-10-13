"""Verify HTTPS identity, certificate lifetime and a bounded health response."""
import argparse
import http.client
import json
import math
import ssl
import time
from urllib.parse import urlsplit

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--url',required=True)
    p.add_argument('--cafile',help='Optional private CA trust file; verification remains enabled')
    p.add_argument('--min-days',type=float,default=14)
    p.add_argument('--timeout',type=float,default=10)
    a=p.parse_args()
    try:
        u=urlsplit(a.url)
        if u.scheme!='https' or not u.hostname or u.username or u.password or u.fragment:
            p.error('--url must be HTTPS without credentials/fragment')
        port=u.port if u.port is not None else 443
        if not 1<=port<=65535:p.error('invalid port')
    except ValueError:p.error('invalid HTTPS URL')
    if not math.isfinite(a.min_days) or a.min_days<0 or not math.isfinite(a.timeout) or a.timeout<=0:
        p.error('invalid certificate/timeout budget')
    connection=None
    try:
        context=ssl.create_default_context(cafile=a.cafile)
        connection=http.client.HTTPSConnection(u.hostname,port,timeout=a.timeout,context=context)
        connection.connect()
        certificate=connection.sock.getpeercert()
        days=(ssl.cert_time_to_seconds(certificate['notAfter'])-time.time())/86400
        if days<a.min_days:
            print(json.dumps({'status':'expires_soon','days_remaining':round(days,3),'required_days':a.min_days}));return 1
        connection.request('GET',(u.path or '/')+('?' + u.query if u.query else ''),headers={'Connection':'close'})
        response=connection.getresponse()
        data=response.read(4097)
        if response.status!=200 or len(data)>4096:
            print(json.dumps({'status':'unhealthy','http_status':response.status}));return 1
        print(json.dumps({'status':'ok','days_remaining':round(days,3),'http_status':response.status}));return 0
    except (OSError,http.client.HTTPException,ValueError,KeyError) as error:
        print(json.dumps({'status':'failed','error':str(error)}));return 1
    finally:
        if connection:connection.close()

if __name__=='__main__':raise SystemExit(main())
