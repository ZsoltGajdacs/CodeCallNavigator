package codegraph.bytecode;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import org.apache.bcel.classfile.*;
import org.apache.bcel.generic.*;

/** Symbolic executable class model: debug attributes and constant-pool order carry no binding evidence. */
final class CompiledClassModel {
    private final JavaClass type;
    private final ConstantPool pool;
    private final BootstrapMethod[] bootstraps;
    private final Map<Integer, Object> constants = new HashMap<>();
    private final java.util.Set<Integer> resolving = new java.util.HashSet<>();

    private CompiledClassModel(JavaClass type) {
        this.type = type;
        pool = type.getConstantPool();
        bootstraps = Arrays.stream(type.getAttributes()).filter(BootstrapMethods.class::isInstance)
                .map(BootstrapMethods.class::cast).map(BootstrapMethods::getBootstrapMethods)
                .findFirst().orElse(new BootstrapMethod[0]);
    }

    static Object of(JavaClass type) {
        return new CompiledClassModel(type).model();
    }

    private Object model() {
        return List.of(type.getMajor(), type.getMinor(), type.getClassName(), type.getSuperclassName(),
                type.getAccessFlags(), Arrays.asList(type.getInterfaceNames()), fields(), methods());
    }

    private Map<String, Object> fields() {
        var fields = new TreeMap<String, Object>();
        for (var field : type.getFields()) {
            var value = field.getConstantValue();
            putMember(fields, field, value == null ? List.of() : constant(value.getConstantValueIndex()));
        }
        return fields;
    }

    private Map<String, Object> methods() {
        var methods = new TreeMap<String, Object>();
        for (var method : type.getMethods()) {
            var exceptions = method.getExceptionTable();
            putMember(methods, method, List.of(
                    exceptions == null ? List.of() : Arrays.asList(exceptions.getExceptionNames()), code(method)));
        }
        return methods;
    }

    private static void putMember(Map<String, Object> members, org.apache.bcel.classfile.FieldOrMethod member, Object body) {
        String key = member.getName() + member.getSignature();
        var model = List.of(member.getAccessFlags(), generic(member), body);
        if (members.putIfAbsent(key, model) != null) throw new IllegalArgumentException("Duplicate member identity");
    }

    private static String generic(org.apache.bcel.classfile.FieldOrMethod member) {
        String value = member.getGenericSignature();
        return value == null ? "" : value;
    }

    private Object code(Method method) {
        var code = method.getCode();
        if (code == null) return List.of();
        var instructions = new InstructionList(code.getCode());
        try {
            var positions = new HashMap<Integer, Integer>();
            var handles = instructions.getInstructionHandles();
            for (int index = 0; index < handles.length; index++) positions.put(handles[index].getPosition(), index);
            positions.put(code.getCode().length, handles.length);
            var body = new ArrayList<Object>();
            for (var handle : handles) body.add(instruction(handle.getInstruction(), positions));
            var handlers = new ArrayList<Object>();
            for (var handler : code.getExceptionTable()) handlers.add(List.of(
                    position(positions, handler.getStartPC()), position(positions, handler.getEndPC()),
                    position(positions, handler.getHandlerPC()),
                    handler.getCatchType() == 0 ? List.of() : constant(handler.getCatchType())));
            return List.of(code.getMaxStack(), code.getMaxLocals(), body, handlers);
        } finally {
            instructions.dispose();
        }
    }

    private Object instruction(Instruction instruction, Map<Integer, Integer> positions) {
        String name = instruction.getName();
        if (instruction instanceof Select select) {
            return List.of(name, position(positions, select.getTarget().getPosition()),
                    Arrays.stream(select.getMatchs()).boxed().toList(),
                    Arrays.stream(select.getTargets()).map(target -> position(positions, target.getPosition())).toList());
        }
        if (instruction instanceof BranchInstruction branch) {
            return List.of(name, position(positions, branch.getTarget().getPosition()));
        }
        if (instruction instanceof CPInstruction reference) {
            if (instruction instanceof LDC) name = "ldc";
            int extra = instruction instanceof MULTIANEWARRAY array ? array.getDimensions()
                    : instruction instanceof INVOKEINTERFACE call ? call.getCount() : 0;
            return List.of(name, constant(reference.getIndex()), extra);
        }
        return instruction.toString(false);
    }

    private static int position(Map<Integer, Integer> positions, int byteOffset) {
        Integer position = positions.get(byteOffset);
        if (position == null) throw new IllegalArgumentException("Invalid instruction boundary");
        return position;
    }

    private Object constant(int index) {
        if (constants.containsKey(index)) return constants.get(index);
        if (!resolving.add(index)) throw new IllegalArgumentException("Cyclic bootstrap constant");
        Object result = value(pool.getConstant(index));
        resolving.remove(index);
        constants.put(index, result);
        return result;
    }

    private Object value(Constant value) {
        if (value instanceof ConstantCP reference) return reference(reference);
        if (value instanceof ConstantNameAndType name) return List.of(value.getTag(), constant(name.getNameIndex()), constant(name.getSignatureIndex()));
        if (value instanceof ConstantMethodHandle handle) return List.of(value.getTag(), handle.getReferenceKind(), constant(handle.getReferenceIndex()));
        if (value instanceof ConstantMethodType method) return List.of(value.getTag(), constant(method.getDescriptorIndex()));
        if (value instanceof ConstantClass name) return List.of(value.getTag(), constant(name.getNameIndex()));
        if (value instanceof ConstantString text) return List.of(value.getTag(), constant(text.getStringIndex()));
        return scalar(value);
    }

    private Object reference(ConstantCP reference) {
        if (reference instanceof ConstantInvokeDynamic dynamic) return dynamic(reference.getTag(), dynamic.getNameAndTypeIndex(), dynamic.getBootstrapMethodAttrIndex());
        if (reference instanceof ConstantDynamic dynamic) return dynamic(reference.getTag(), dynamic.getNameAndTypeIndex(), dynamic.getBootstrapMethodAttrIndex());
        return List.of(reference.getTag(), constant(reference.getClassIndex()), constant(reference.getNameAndTypeIndex()));
    }

    private static Object scalar(Constant value) {
        if (value instanceof ConstantUtf8 text) return List.of(value.getTag(), text.getBytes());
        if (value instanceof ConstantInteger number) return List.of(value.getTag(), number.getBytes());
        if (value instanceof ConstantLong number) return List.of(value.getTag(), number.getBytes());
        if (value instanceof ConstantFloat number) return List.of(value.getTag(), Float.floatToRawIntBits(number.getBytes()));
        if (value instanceof ConstantDouble number) return List.of(value.getTag(), Double.doubleToRawLongBits(number.getBytes()));
        throw new IllegalArgumentException("Unsupported executable constant: " + value.getTag());
    }

    private Object dynamic(int tag, int name, int bootstrapIndex) {
        var bootstrap = bootstraps[bootstrapIndex];
        return List.of(tag, constant(name), constant(bootstrap.getBootstrapMethodRef()),
                Arrays.stream(bootstrap.getBootstrapArguments()).mapToObj(this::constant).toList());
    }
}
