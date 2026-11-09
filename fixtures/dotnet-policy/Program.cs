
using System;
class Program {
 static void Main() { new Worker().Start(); var entry = new Entry(1); _ = entry.Number; _ = entry.ToString(); _ = entry.GetHashCode(); _ = entry.Equals(entry); entry.Authored(); }
}
interface IObjectLike {
 string ToString();
 int GetHashCode();
 bool Equals(object other);
 string ToString(int value);
}
class Explicit : IObjectLike {
 string IObjectLike.ToString() => "explicit";
 int IObjectLike.GetHashCode() => 1;
 bool IObjectLike.Equals(object other) => false;
 string IObjectLike.ToString(int value) => "overload";
}
class Worker {
 public Worker() { ConstructorOnly(); }
 static Worker() { ConstructorOnly(); }
 public override string ToString() => "worker";
 public string ToString(int value) => "overload";
 public override int GetHashCode() => 1;
 public int GetHashCode(int salt) => salt;
 public override bool Equals(object value) => false;
 public bool Equals(int value) => false;
 public void Start() { Private(); Package(); Protected(); Action run = () => Private(); run(); _ = Value; }
 private void Private() {}
 internal void Package() {}
 protected void Protected() {}
 public int Value { get { Private(); return 1; } }
 public int Automatic { get; set; }
 public void UseAutomatic() { Automatic = 1; _ = Automatic; }
 private static void ConstructorOnly() {}
}
record Entry(int Number) { public void Authored() {} }
