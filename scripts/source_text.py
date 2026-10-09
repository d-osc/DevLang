"""Keep generated legacy test/benchmark programs explicit about their entry call."""
import re

def explicit_entry(source, path=None):
    # Fixture generators predate top-level execution. Add a literal entry call
    # to their generated source, preserving tests that check main's exit status.
    if path is not None and path.suffix != '.dev':
        return source
    if re.search(r'\bfn\s+main\s*\(', source) and not re.search(r'^\s*(?:return\s+)?main\(\)\s*;?\s*$', source, re.M):
        source += '\nreturn main()\n'
    return unsafe_fixture(source)


def unsafe_fixture(source):
    """Migrate legacy foreign-memory fixtures explicitly; never used by safety tests."""
    tokens=list(re.finditer(r'"(?:\\.|[^"\\])*"|\#.*|//[^\n]*|[A-Za-z_$][\w$]*|\n|[^\s]',source))
    tokens=[t for t in tokens if not t.group().startswith(('#','//'))]
    declarations=[];statements=[];at=0
    def skip(at):
        while at<len(tokens) and tokens[at].group() in ('\n',';'):at+=1
        return at
    while (at:=skip(at))<len(tokens):
        start=at;word=tokens[at].group();decl=word in ('fn','async','extern','export','struct','enum','trait','use')
        braces=parens=brackets=0;body_start=None;external=word=='extern';is_function=word in ('fn','async','extern','export')
        at+=1
        while at<len(tokens):
            text=tokens[at].group()
            if text=='(':parens+=1
            elif text==')':parens-=1
            elif text=='[':brackets+=1
            elif text==']':brackets-=1
            elif text=='{':
                if braces==0 and is_function:body_start=at
                braces+=1
            elif text=='}':
                braces-=1
                if braces==0 and parens==brackets==0:
                    at+=1
                    following=skip(at)
                    if not decl and following<len(tokens) and tokens[following].group()=='else':at=following;continue
                    break
            elif text in (';','\n') and braces==parens==brackets==0:
                if not is_function or external:at+=1;break
            at+=1
        end=tokens[at-1].end();begin=tokens[start].start()
        chunk=source[begin:end]
        if decl:
            if is_function and body_start is not None:
                pos=tokens[body_start].end()-begin
                close=chunk.rfind('}')
                chunk=chunk[:pos]+' unsafe { '+chunk[pos:close]+' } '+chunk[close:]
            declarations.append(chunk)
        else:statements.append(chunk)
    return '\n'.join(declarations)+ ('\nunsafe {\n'+'\n'.join(statements)+'\n}\n' if statements else '\n')
